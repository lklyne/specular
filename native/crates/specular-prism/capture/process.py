#!/usr/bin/env python3
"""Turn reference/_raw (the capture) into the reference/ deliverables.

Run with the scratch venv (numpy + pillow):  venv/bin/python process.py
"""
import json, os, re, shutil, struct, sys
from pathlib import Path
import numpy as np
from PIL import Image

REF = Path(__file__).resolve().parent.parent / 'reference'
RAW = REF / '_raw'
BG = (0xE9, 0xE5, 0xDC)  # the page background the canvas composites over

O = {o['id']: o for o in json.load(open(RAW / 'init/objects.json'))}


# hook slots in scenes/prism.tsx / scene/logo-scene.tsx, in declaration order
LABEL_PREFIXES = [
    ('PrismScene.hook8.', 'scene.'),            # createSceneUniforms (rim + stylized specular)
    ('PrismScene.hook11.', 'brush.'),           # paintedNormalEffect uniforms
    ('PrismScene.hook342.', 'paint.'),          # createPaintUniforms (post/paint-settings.ts)
    ('PrismScene.hook367.', 'grade.'),          # createGradeUniforms (post/grade.tsx)
    ('GradientCard.hook0.uniforms.', 'gradient.'),
    ('GradientCard.hook0.colorsUniform', 'gradient.colors'),
]


def pretty_label(l):
    for a, b in LABEL_PREFIXES:
        if l.startswith(a):
            return b + l[len(a):]
    return l


def load_frame(tag):
    d = RAW / 'frames' / tag
    tr = json.load(open(d / 'trace.json'))
    objs = {o['id']: o for o in json.load(open(d / 'objects.json'))}
    three = json.load(open(d / 'three-uniforms.json')) if (d / 'three-uniforms.json').exists() else []
    return d, tr, objs, three


# ---------------------------------------------------------------- WGSL layout
SCALARS = {'f32': (4, 4), 'i32': (4, 4), 'u32': (4, 4)}


def type_layout(t):
    t = t.replace(' ', '')
    if t in SCALARS:
        return SCALARS[t]
    m = re.fullmatch(r'vec([234])<(f32|i32|u32)>', t)
    if m:
        n = int(m.group(1))
        return {2: (8, 8), 3: (16, 12), 4: (16, 16)}[n]
    m = re.fullmatch(r'mat([234])x([234])<f32>', t)
    if m:
        c, r = int(m.group(1)), int(m.group(2))
        al, _ = type_layout(f'vec{r}<f32>')
        return al, al * c
    m = re.fullmatch(r'array<(.+),(\d+)>', t)
    if m:
        al, sz = type_layout(m.group(1))
        stride = (sz + al - 1) // al * al
        return al, stride * int(m.group(2))
    raise ValueError('unknown type ' + t)


def parse_structs(code):
    out = {}
    for m in re.finditer(r'struct\s+(\w+)\s*\{(.*?)\};', code, re.S):
        members = []
        body, parts, depth, cur = m.group(2), [], 0, ''
        for ch in body:
            if ch == '<':
                depth += 1
            elif ch == '>':
                depth -= 1
            if ch == ',' and depth == 0:
                parts.append(cur)
                cur = ''
            else:
                cur += ch
        parts.append(cur)
        for line in parts:
            line = line.strip()
            if not line or line.startswith('@'):
                continue
            mm = re.match(r'(\w+)\s*:\s*(.+)$', line)
            if mm:
                members.append((mm.group(1), mm.group(2).strip()))
        out[m.group(1)] = members
    return out


def parse_bindings(code):
    """(group, binding) -> (kind, name, type)"""
    out = {}
    for m in re.finditer(r'@binding\(\s*(\d+)\s*\)\s*@group\(\s*(\d+)\s*\)\s*var(<uniform>|<storage[^>]*>)?\s+(\w+)\s*:\s*([^;]+);', code):
        out[(int(m.group(2)), int(m.group(1)))] = (m.group(3) or 'handle', m.group(4), m.group(5).strip())
    return out


def decode_struct(members, buf, structs):
    vals, off = {}, 0
    for name, t in members:
        al, sz = type_layout(t)
        off = (off + al - 1) // al * al
        vals[name] = decode_value(t.replace(' ', ''), buf, off)
        off += sz
    return vals


def f32s(buf, off, n):
    return list(struct.unpack_from(f'<{n}f', buf, off))


def decode_value(t, buf, off):
    if off + type_layout(t)[1] > len(buf):
        return None
    if t == 'f32':
        return f32s(buf, off, 1)[0]
    if t in ('i32',):
        return struct.unpack_from('<i', buf, off)[0]
    if t in ('u32',):
        return struct.unpack_from('<I', buf, off)[0]
    m = re.fullmatch(r'vec([234])<f32>', t)
    if m:
        return f32s(buf, off, int(m.group(1)))
    m = re.fullmatch(r'mat([234])x([234])<f32>', t)
    if m:
        c, r = int(m.group(1)), int(m.group(2))
        al, _ = type_layout(f'vec{r}<f32>')
        return [f32s(buf, off + i * al, r) for i in range(c)]  # column-major
    m = re.fullmatch(r'array<(.+),(\d+)>', t)
    if m:
        al, sz = type_layout(m.group(1))
        stride = (sz + al - 1) // al * al
        return [decode_value(m.group(1), buf, off + i * stride) for i in range(int(m.group(2)))]
    return None


def clean(v):
    if isinstance(v, float):
        if v != v:
            return 'NaN'
        if v in (float('inf'), float('-inf')):
            return 'Infinity' if v > 0 else '-Infinity'
        return round(v, 7)
    if isinstance(v, list):
        return [clean(x) for x in v]
    if isinstance(v, dict):
        return {k: clean(x) for k, x in v.items()}
    return v


# ---------------------------------------------------------------- images
def rows(data, w, h, bpr, bpp):
    a = np.frombuffer(data, dtype=np.uint8).reshape(h, bpr)[:, : w * bpp]
    return np.ascontiguousarray(a)


def to_float(data, w, h, bpr, fmt):
    if fmt == 'rgba16float':
        return rows(data, w, h, bpr, 8).view(np.float16).reshape(h, w, 4).astype(np.float32)
    if fmt == 'rg16float':
        return rows(data, w, h, bpr, 4).view(np.float16).reshape(h, w, 2).astype(np.float32)
    if fmt == 'rgba32float':
        return rows(data, w, h, bpr, 16).view(np.float32).reshape(h, w, 4)
    if fmt in ('rgba8unorm', 'rgba8unorm-srgb'):
        return rows(data, w, h, bpr, 4).reshape(h, w, 4).astype(np.float32) / 255
    if fmt in ('bgra8unorm', 'bgra8unorm-srgb'):
        return rows(data, w, h, bpr, 4).reshape(h, w, 4)[..., [2, 1, 0, 3]].astype(np.float32) / 255
    raise ValueError(fmt)


def save_png(arr, path, premultiplied=False, over_bg=False, srgb_encode=False):
    a = arr.copy()
    if a.shape[2] == 2:
        a = np.concatenate([a, np.zeros_like(a[..., :1]), np.ones_like(a[..., :1])], axis=2)
    rgb, al = a[..., :3], np.clip(a[..., 3:4], 0, 1)
    if over_bg:
        bg = np.array(BG, np.float32) / 255
        if not premultiplied:
            rgb = rgb * al
        rgb = rgb + bg * (1 - al)
        al = np.ones_like(al)
    elif premultiplied:
        rgb = np.where(al > 1e-6, rgb / np.maximum(al, 1e-6), 0)
    if srgb_encode:
        rgb = np.where(rgb <= 0.0031308, rgb * 12.92, 1.055 * np.power(np.maximum(rgb, 0), 1 / 2.4) - 0.055)
    out = np.concatenate([np.clip(rgb, 0, 1), al], axis=2)
    Image.fromarray((out * 255 + 0.5).astype(np.uint8), 'RGBA').save(path)


def save_raw(arr, path):
    """Tightly packed little-endian float16 RGBA (or RG), row 0 = top of the texture.
    Lossless for the half-float targets; 8-bit sources are exact in f16 too."""
    np.ascontiguousarray(arr.astype('<f2')).tofile(path)


# ---------------------------------------------------------------- pass graph
def view_tex(objs, vid):
    v = objs.get(vid) or O.get(vid)
    if not v:
        return None, None
    return v['texture'], v.get('desc')


def tex_info(objs, tid):
    t = objs.get(tid) or O.get(tid)
    return t


def build_passes(tr, objs, three):
    """List of passes in submission order with their draws."""
    pipe_to_ro = {}
    for e in three:
        if e.get('pipelineGpuId'):
            pipe_to_ro.setdefault(e['pipelineGpuId'], e)
    passes, by_pass = [], {}
    cur_bind = {}
    for t in tr['trace']:
        op = t['op']
        if op == 'beginRenderPass':
            p = {'index': t['passIndex'], 'colors': t['colors'], 'depth': t['depth'], 'draws': [], 'viewport': None, 'ended': False, 'pre': []}
            passes.append(p)
            by_pass[t['pass']] = p
            cur_bind[t['pass']] = {'pipeline': None, 'groups': {}, 'vb': {}, 'ib': None}
        elif op == 'setPipeline':
            cur_bind[t['pass']]['pipeline'] = t['pipeline']
        elif op == 'setBindGroup':
            cur_bind[t['pass']]['groups'][t['index']] = t['group']
        elif op == 'setVertexBuffer':
            cur_bind[t['pass']]['vb'][t['slot']] = (t['buffer'], t.get('offset'), t.get('size'))
        elif op == 'setIndexBuffer':
            cur_bind[t['pass']]['ib'] = (t['buffer'], t['format'], t.get('offset'), t.get('size'))
        elif op == 'setViewport':
            by_pass[t['pass']]['viewport'] = t['args']
        elif op in ('draw', 'drawIndexed'):
            st = cur_bind[t['pass']]
            ro = pipe_to_ro.get(st['pipeline'])
            by_pass[t['pass']]['draws'].append({
                'op': op, 'args': t['args'], 'pipeline': st['pipeline'], 'groups': dict(st['groups']),
                'vb': dict(st['vb']), 'ib': st['ib'],
                'material': ro and ro['material'], 'object': ro and ro['object'],
            })
        elif op == 'endRenderPass':
            by_pass[t['pass']]['ended'] = True
        elif op == 'copyTextureToTexture':
            # attach to the most recently ended pass
            passes[-1].setdefault('after', []).append(t)
    return passes, pipe_to_ro


def main():
    for sub in ['wgsl', 'uniforms', 'textures', 'geometry', 'pass-outputs', 'pipelines']:
        p = REF / sub
        if p.exists():
            shutil.rmtree(p)
        p.mkdir(parents=True)

    main_tag = 'f0240'
    d, tr, objs, three = load_frame(main_tag)
    allobjs = {**O, **objs}
    passes, pipe_to_ro = build_passes(tr, allobjs, three)

    # ---- name each frame pipeline after its pass
    pipe_names, order = {}, 0
    draw_order = []
    cur_pipe = {}
    for t in tr['trace']:
        if t['op'] == 'setPipeline':
            cur_pipe[t['pass']] = t['pipeline']
        elif t['op'] in ('draw', 'drawIndexed'):
            draw_order.append(cur_pipe[t['pass']])
    for pid in draw_order:
        if True:
            dr = {'pipeline': pid, 'material': None}
            if dr['pipeline'] in pipe_names:
                continue
            po = allobjs[dr['pipeline']]
            label = po['label'].replace('renderPipeline_', '')
            label = re.sub(r'_\d+$', '', label)
            mat = dr['material']['name'] if dr['material'] else ''
            pipe_names[dr['pipeline']] = f'{order:02d}-{label}'
            order += 1

    # init-only pipelines that still matter (PMREM, mipmaps, the pre-scene output transform)
    init_trace = json.load(open(RAW / 'init/trace.json'))
    init_pipes = []
    for t in init_trace:
        if t['op'] == 'setPipeline' and t['pipeline'] not in init_pipes and t['pipeline'] not in pipe_names:
            init_pipes.append(t['pipeline'])
    keep_init = [p for p in init_pipes if re.search(r'PMREM|mipmap', O[p]['label'])]
    for p in keep_init:
        label = re.sub(r'_\d+$', '', O[p]['label'].replace('renderPipeline_', ''))
        pipe_names[p] = f'init-{label}'

    # static RTTs (autoUpdate=false): bound every frame, rendered once at init
    written = set()
    for t in tr['trace']:
        if t['op'] == 'beginRenderPass':
            written.update(view_tex(allobjs, c['view'])[0] for c in t['colors'] if c)
        if t['op'] == 'copyTextureToTexture':
            written.add(t['dst'])
    bound_by = {}
    for p in passes:
        for dr in p['draws']:
            for gid in dr['groups'].values():
                for e in allobjs[gid]['entries']:
                    if 'textureView' in e:
                        bound_by.setdefault(view_tex(allobjs, e['textureView'])[0], set()).add(dr['pipeline'])
    last_writer, cur, tgt = {}, {}, {}
    for t in init_trace:
        if t['op'] == 'beginRenderPass':
            tgt[t['pass']] = [O.get(c['view'], {}).get('texture') for c in t['colors'] if c]
        elif t['op'] == 'setPipeline':
            cur[t['pass']] = t['pipeline']
        elif t['op'].startswith('draw'):
            for tx in tgt.get(t['pass'], []):
                last_writer[tx] = cur.get(t['pass'])
    composite_pipe = draw_order[-1]
    static_rtts = {}
    for tex, users in bound_by.items():
        lab = (allobjs.get(tex) or {}).get('label', '')
        if tex in written or lab in ('DFG_LUT', 'PMREM.cubeUv', 'BrushNormal') or tex not in last_writer:
            continue
        kind = 'paper-frame' if composite_pipe in users else 'stroke-noise'
        pipe_names[last_writer[tex]] = f'static-{kind}'
        static_rtts[kind] = {'texId': tex, 'pipeline': last_writer[tex]}

    # ---- WGSL + pipeline state
    shader_files = {}
    for pid, name in pipe_names.items():
        po = allobjs[pid]
        for stage, key in (('vs', 'vertex'), ('fs', 'fragment')):
            s = po.get(key)
            if not s:
                continue
            mod = allobjs[s['module']]
            fn = f'{name}-{stage}.wgsl'
            (REF / 'wgsl' / fn).write_text(mod['code'])
            fixed = re.sub(r'smoothstep\( ([\d.]+), ([\d.]+), ([\d.]+) \)', r'smoothstep( \1, \2, f32( \3 ) )', mod['code'])
            if fixed != mod['code']:
                (REF / 'wgsl' / fn.replace('.wgsl', '.naga.wgsl')).write_text(fixed)
            shader_files.setdefault(pid, {})[stage] = fn
        layout = po['layout']
        bgls = []
        if layout != 'auto' and layout in allobjs:
            for bid in allobjs[layout]['bindGroupLayouts']:
                bgls.append(allobjs[bid].get('entries'))
        state = {k: po.get(k) for k in ('label', 'primitive', 'depthStencil', 'multisample')}
        state['vertexBuffers'] = po['vertex'].get('buffers')
        state['targets'] = po['fragment'].get('targets') if po.get('fragment') else None
        state['bindGroupLayouts'] = bgls
        state['shaders'] = shader_files.get(pid)
        (REF / 'pipelines' / f'{name}.json').write_text(json.dumps(state, indent=1))

    # ---- uniforms per draw, for every captured frame
    frame_tags = sorted(os.listdir(RAW / 'frames'))
    for tag in frame_tags:
        fd, ftr, fobjs, fthree = load_frame(tag)
        fall = {**O, **fobjs}
        fpasses, fp2ro = build_passes(ftr, fall, fthree)
        three_names = {}
        for e in fthree:
            for g in e['groups']:
                for b in g['bindings']:
                    for u in b.get('uniforms') or []:
                        three_names.setdefault(e.get('pipelineGpuId'), {})[u['name']] = u['node'] or {}
        out = []
        for p in fpasses:
            for dr in p['draws']:
                po = fall[dr['pipeline']]
                codes = [fall[po['vertex']['module']]['code']]
                if po.get('fragment'):
                    codes.insert(0, fall[po['fragment']['module']]['code'])
                bindings, structs = {}, {}
                for c in codes:
                    for k, v in parse_bindings(c).items():
                        bindings.setdefault(k, v)
                    for k, v in parse_structs(c).items():
                        structs.setdefault(k, v)
                draw = {'pass': p['index'], 'pipeline': pipe_names.get(dr['pipeline'], po['label']),
                        'material': dr['material'] and dr['material']['name'] or (dr['material'] and dr['material']['type']),
                        'groups': {}}
                for gi, gid in sorted(dr['groups'].items()):
                    g = fall[gid]
                    ge = {}
                    for e in g['entries']:
                        key = (int(gi), e['binding'])
                        kind, name, typ = bindings.get(key, ('?', f'binding{e["binding"]}', '?'))
                        if 'buffer' in e:
                            bpath = fd / 'buffers' / f'{e["buffer"]}.bin'
                            buf = bpath.read_bytes() if bpath.exists() else b''
                            buf = buf[e.get('offset', 0):]
                            members = structs.get(typ)
                            if members:
                                vals = decode_struct(members, buf, structs)
                                named = {}
                                tn = three_names.get(dr['pipeline'], {})
                                for mname, v in vals.items():
                                    meta = tn.get(mname, {})
                                    named[mname] = {'value': clean(v), 'type': dict(members)[mname]}
                                    lab = meta.get('ref') or (meta.get('label') and pretty_label(meta['label'])) or (meta.get('name') if meta.get('name') != mname else None)
                                    if lab:
                                        named[mname]['source'] = lab
                                ge[name] = {'kind': 'uniform', 'struct': typ, 'members': named}
                            else:
                                ge[name] = {'kind': 'buffer', 'bytes': len(buf)}
                        elif 'textureView' in e:
                            tid, vdesc = view_tex(fall, e['textureView'])
                            ti = tex_info(fall, tid) or {}
                            ge[name] = {'kind': 'texture', 'texId': tid, 'label': ti.get('label'), 'size': ti.get('size'), 'format': ti.get('format'), 'mips': ti.get('mipLevelCount'), 'view': vdesc}
                        elif 'sampler' in e:
                            ge[name] = {'kind': 'sampler', 'desc': fall[e['sampler']]['desc']}
                    draw['groups'][str(gi)] = ge
                out.append(draw)
        (REF / 'uniforms' / f'{tag}.json').write_text(json.dumps(out, indent=1))

    for kind, info in static_rtts.items():
        ro = next((e for e in three if e.get('pipelineGpuId') == info['pipeline']), None)
        vals = {}
        if ro:
            for g in ro['groups']:
                for b in g['bindings']:
                    for u in b.get('uniforms') or []:
                        nd = u['node'] or {}
                        vals[u['name']] = {'value': clean(u['value']), 'type': u['type'], 'source': nd.get('ref') or (nd.get('label') and pretty_label(nd['label']))}
                    if b['type'] == 'NodeSampledTexture':
                        vals[b['name']] = {'texture': b.get('texture')}
        (REF / 'uniforms' / f'static-{kind}.json').write_text(json.dumps({'note': 'three-side JS values (rendered once at init; GPU buffer not captured)', 'texId': info['texId'], 'uniforms': vals}, indent=1))
    (REF / 'pass-graph-static.json').write_text(json.dumps(static_rtts, indent=1))

    # ---- pass outputs (snapshots) -> PNG + raw float
    for tag in frame_tags:
        fd, ftr, fobjs, fthree = load_frame(tag)
        snaps = ftr.get('snaps', [])
        if not snaps:
            continue
        od = REF / 'pass-outputs' / tag
        od.mkdir(parents=True, exist_ok=True)
        full_raw = tag in ('f0240', 'f0480-s128', 'f0480-s64')
        for s in snaps:
            f = fd / 'snaps' / s['file']
            if not f.exists():
                continue
            arr = to_float(f.read_bytes(), s['w'], s['h'], s['bytesPerRow'], s['format'])
            base = f"pass{s['passIndex']:02d}-tex{s['texId']}{'-canvas' if s['canvas'] else ''}"
            save_png(arr, od / f'{base}.png', premultiplied=True)
            if s['canvas']:
                save_png(arr, od / f'{base}-over-bg.png', premultiplied=True, over_bg=True)
            if full_raw or (s['canvas'] and (s['w'] < 1024 or tag in ('f0000', 'f0240', 'f0480'))):
                save_raw(arr, od / f'{base}.{s["w"]}x{s["h"]}.rgba16f')

    # ---- reference shots
    shots = REF / 'shots'
    shots.mkdir(exist_ok=True)
    for f in shots.glob('f*.png'):
        f.unlink()
    manifest = {'note': 'Canvas readbacks (rgba16float, premultiplied, values written straight to an sRGB canvas: no encode). '
                        '*-over-bg composites over the page bg #e9e5dc; *-alpha keeps coverage in alpha (un-premultiplied). '
                        'The composite clamps to [0,1] before the canvas, so the extended-range canvas never holds >1 here. '
                        'Time: virtual 60 Hz clock, frame N = N/60 s since mount, all scene clocks started at 0.', 'shots': []}
    for tag in frame_tags:
        fd, ftr, _, _ = load_frame(tag)
        for s in ftr.get('snaps', []):
            if not s['canvas']:
                continue
            arr = to_float((fd / 'snaps' / s['file']).read_bytes(), s['w'], s['h'], s['bytesPerRow'], s['format'])
            m = re.match(r'f(\d+)(?:-s(\d+))?', tag)
            frame = int(m.group(1))
            for suffix, kw in (('over-bg', dict(premultiplied=True, over_bg=True)), ('alpha', dict(premultiplied=True))):
                fn = f'{tag}-{s["w"]}-{suffix}.png'
                save_png(arr, shots / fn, **kw)
                manifest['shots'].append({'file': fn, 'size': s['w'], 'frame': frame, 'timeSeconds': round(frame / 60, 4), 'renderedAt': s['w'], 'kind': suffix})
            if s['w'] == 1024 and tag in ('f0240', 'f0480'):
                for small in (128, 64):
                    for suffix in ('over-bg', 'alpha'):
                        im = Image.open(shots / f'{tag}-1024-{suffix}.png').resize((small, small), Image.LANCZOS)
                        fn = f'{tag}-1024-downscaled-{small}-{suffix}.png'
                        im.save(shots / fn)
                        manifest['shots'].append({'file': fn, 'size': small, 'frame': frame, 'timeSeconds': round(frame / 60, 4), 'renderedAt': 1024, 'kind': suffix + ' (Lanczos downscale)'})
    for f in sorted(shots.glob('browser-composited-*.png')):
        frame = int(re.search(r'f(\d+)', f.name).group(1))
        manifest['shots'].append({'file': f.name, 'size': 1024, 'frame': frame, 'timeSeconds': round(frame / 60, 4), 'renderedAt': 1024, 'kind': 'CDP screenshot of the page (Chrome compositing + colour management); pixel-identical to the matching *-over-bg readback'})
    (shots / 'manifest.json').write_text(json.dumps(manifest, indent=1))

    # ---- input textures
    man = json.load(open(RAW / 'textures/manifest.json'))
    if (RAW / 'textures/static-manifest.json').exists():
        man += json.load(open(RAW / 'textures/static-manifest.json'))
    for t in man:
        data = (RAW / 'textures' / f"{t['name']}-mip{t['mip']}.bin").read_bytes()
        arr = to_float(data, t['w'], t['h'], t['bytesPerRow'], t['format'])
        if t['name'].startswith('static'):
            # stroke noise writes w = 0; the paper frame writes grain * 0.5 + 0.5 there
            t['name'] = 'static-stroke-noise' if float(np.abs(arr[..., 3]).max()) == 0 else 'static-paper-frame'
        base = f"{t['name']}-mip{t['mip']}"
        save_raw(arr, REF / 'textures' / f"{base}.{t['w']}x{t['h']}.{'rg' if arr.shape[2] == 2 else 'rgba'}16f")
        if t['name'].startswith('static'):
            prev = arr.copy(); prev[..., 3] = 1
            save_png(prev, REF / 'textures' / f'{base}.preview.png')
            continue
        if t['format'] == 'rgba8unorm':
            Image.fromarray((arr * 255 + .5).astype(np.uint8), 'RGBA').save(REF / 'textures' / f'{base}.png')
        else:
            prev = arr.copy()
            if prev.shape[2] == 4:
                prev[..., 3] = 1
            save_png(prev / max(1e-6, float(np.percentile(prev[..., :3], 99.5))) if t['name'].startswith(('pmrem', 'env')) else prev,
                     REF / 'textures' / f'{base}.preview.png', srgb_encode=t['name'].startswith(('pmrem', 'env')))
    shutil.copy(RAW / 'textures/manifest.json', REF / 'textures/readback-manifest.json')

    # ---- geometry: vertex + index buffers of the scene draws
    geo = {}
    for p in passes:
        for dr in p['draws']:
            po = allobjs[dr['pipeline']]
            name = pipe_names[dr['pipeline']]
            if not dr['ib']:
                # fullscreen triangle: position comes from vertex_index, only the uv buffer is bound
                bid = dr['vb'][0][0] if 0 in dr['vb'] else None
                if bid is not None and 'fullscreen-triangle' not in geo:
                    shutil.copy(d / 'buffers' / f'{bid}.bin', REF / 'geometry' / 'fullscreen-triangle-uv.f32')
                    geo['fullscreen-triangle'] = {'drawArgs': dr['args'], 'usedBy': 'every post pass', 'attributes': [{'slot': 0, 'file': 'fullscreen-triangle-uv.f32', 'layout': po['vertex']['buffers'][0], 'values': list(struct.unpack('<6f', (d / 'buffers' / f'{bid}.bin').read_bytes()[:24]))}]}
                continue
            entry = {'drawArgs': dr['args'], 'attributes': []}
            for slot, (bid, off, size) in sorted(dr['vb'].items()):
                src = d / 'buffers' / f'{bid}.bin'
                lay = po['vertex']['buffers'][slot]
                fn = f'{name}-slot{slot}.f32'
                shutil.copy(src, REF / 'geometry' / fn)
                entry['attributes'].append({'slot': slot, 'file': fn, 'layout': lay, 'bytes': src.stat().st_size})
            bid, fmt, off, size = dr['ib']
            fn = f'{name}-index.{fmt}'
            shutil.copy(d / 'buffers' / f'{bid}.bin', REF / 'geometry' / fn)
            entry['index'] = {'file': fn, 'format': fmt, 'bytes': (d / 'buffers' / f'{bid}.bin').stat().st_size}
            geo[name] = entry
    (REF / 'geometry' / 'manifest.json').write_text(json.dumps(geo, indent=1))

    # ---- pass graph summary (machine-readable)
    summary = []
    for p in passes:
        cols = []
        for c in p['colors']:
            if not c:
                continue
            tid, vd = view_tex(allobjs, c['view'])
            ti = tex_info(allobjs, tid) or {}
            cols.append({'texId': tid, 'label': ti.get('label'), 'size': ti.get('size'), 'format': ti.get('format'), 'canvas': bool(ti.get('canvas')), 'loadOp': c['loadOp'], 'clear': c.get('clearValue')})
        dep = None
        if p['depth']:
            tid, _ = view_tex(allobjs, p['depth']['view'])
            ti = tex_info(allobjs, tid) or {}
            dep = {'texId': tid, 'format': ti.get('format'), 'loadOp': p['depth']['depthLoadOp']}
        summary.append({
            'passIndex': p['index'], 'colors': cols, 'depth': dep, 'viewport': p['viewport'],
            'draws': [{'pipeline': pipe_names.get(dr['pipeline']), 'material': dr['material'] and dr['material']['name'], 'object': dr['object'], 'args': dr['args'], 'op': dr['op']} for dr in p['draws']],
            'copiesAfter': [{'src': c['src'], 'dst': c['dst']} for c in p.get('after', [])],
        })
    (REF / 'pass-graph.json').write_text(json.dumps(summary, indent=1))
    print(json.dumps(summary, indent=1)[:200])
    print('pipelines', pipe_names)


if __name__ == '__main__':
    main()
