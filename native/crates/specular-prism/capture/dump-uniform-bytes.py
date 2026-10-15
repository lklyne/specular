#!/usr/bin/env python3
"""Write the raw uniform-buffer bytes each scene/display draw was bound to.

reference/uniform-bytes/<frame>/<pipeline>-g<group>-b<binding>.bin, sliced at the
bound offset and cut to the bound size (or the rest of the buffer). The parity
harness compares the Rust-packed uniforms against these byte for byte.
No numpy needed.
"""
import json
from pathlib import Path

REF = Path(__file__).resolve().parent.parent / 'reference'
RAW = REF / '_raw'
O = {o['id']: o for o in json.load(open(RAW / 'init/objects.json'))}
KEEP = {'00-MeshBasicNodeMaterial', '01-ShellLiquid', '02-ShellGlass', '03-RTT', '11-RenderPipeline'}


def main():
    # pipeline label -> name, as in uniforms/f0240.json (same draw order)
    for tag in sorted(p.name for p in (RAW / 'frames').iterdir()):
        d = RAW / 'frames' / tag
        tr = json.load(open(d / 'trace.json'))
        objs = {**O, **{o['id']: o for o in json.load(open(d / 'objects.json'))}}
        named = json.load(open(REF / 'uniforms' / f'{tag}.json'))
        groups, draws = {}, []
        for t in tr['trace']:
            if t['op'] == 'beginRenderPass':
                groups[t['pass']] = {}
            elif t['op'] == 'setBindGroup':
                groups[t['pass']][t['index']] = t['group']
            elif t['op'] in ('draw', 'drawIndexed'):
                draws.append(dict(groups[t['pass']]))
        assert len(draws) == len(named), (tag, len(draws), len(named))
        out = REF / 'uniform-bytes' / tag
        out.mkdir(parents=True, exist_ok=True)
        for dr, nm in zip(draws, named):
            if nm['pipeline'] not in KEEP:
                continue
            for gi, gid in dr.items():
                for e in objs[gid]['entries']:
                    if 'buffer' not in e:
                        continue
                    b = (d / 'buffers' / f'{e["buffer"]}.bin').read_bytes()
                    off = e.get('offset', 0) or 0
                    size = e.get('size')
                    b = b[off:off + size] if size else b[off:]
                    (out / f'{nm["pipeline"]}-g{gi}-b{e["binding"]}.bin').write_bytes(b)


if __name__ == '__main__':
    main()
