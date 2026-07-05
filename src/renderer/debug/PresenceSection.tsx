/**
 * Presence debug panel — playground on the left, tuning controls on the right.
 *
 * The playground mirrors the director's advance loop locally so the user can
 * retarget the cursor with a click and watch the speed model react to tuning
 * changes in real time. If the playground feels snappy but the real agent
 * cursor feels slow, the problem is somewhere outside the director.
 */

import { useState, type ReactNode } from 'react'
import type {
  CursorTuningParams,
  EasingPreset,
  EasingSpec,
} from '../../shared/types'
import {
  DEFAULT_TRAIL_PARAMS,
  PresencePlayground,
  type TrailFadeEasing,
  type TrailParticleParams,
} from './PresencePlayground'

const EASING_PRESETS: EasingPreset[] = [
  'linear',
  'easeInOutCubic',
  'easeOutExpo',
  'easeInOutQuart',
  'easeOutBack',
  'easeInOutSine',
]

export function PresenceSection({
  splineViz,
  onSplineVizChange,
  tuning,
  onTuningChange,
  onTuningReset,
}: {
  splineViz: boolean
  onSplineVizChange: (on: boolean) => void
  tuning: CursorTuningParams
  onTuningChange: (next: CursorTuningParams) => void
  onTuningReset: () => void
}) {
  const [trail, setTrail] = useState<TrailParticleParams>(DEFAULT_TRAIL_PARAMS)

  return (
    <div className="flex h-full w-full min-w-0">
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="relative min-h-0 flex-1 overflow-hidden">
          <PresencePlayground tuning={tuning} trail={trail} />
        </div>
      </div>
      <div className="w-80 shrink-0 overflow-y-auto border-l border-[var(--surface-popover-border)]">
        <TuningControls
          splineViz={splineViz}
          onSplineVizChange={onSplineVizChange}
          tuning={tuning}
          onTuningChange={onTuningChange}
          onTuningReset={onTuningReset}
        />
        <hr className="border-[var(--surface-popover-border)]" />
        <TrailControls trail={trail} onChange={setTrail} />
      </div>
    </div>
  )
}

function TrailControls({
  trail,
  onChange,
}: {
  trail: TrailParticleParams
  onChange: (next: TrailParticleParams) => void
}) {
  const patch = (p: Partial<TrailParticleParams>) => onChange({ ...trail, ...p })

  return (
    <div className="flex flex-col gap-4 p-4 text-[12px]">
      <Header
        title="Trail particles"
        onReset={() => onChange(DEFAULT_TRAIL_PARAMS)}
      />

      <Field
        label="Particle size"
        value={`${trail.size} px`}
        help="Edge length of each particle quad in screen pixels."
      >
        <input
          type="range"
          min={2}
          max={40}
          step={1}
          value={trail.size}
          onChange={(e) => patch({ size: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Emit offset X"
        value={`${trail.offsetX > 0 ? '+' : ''}${trail.offsetX} px`}
        help="Horizontal offset from the cursor anchor where particles are emitted."
      >
        <input
          type="range"
          min={-30}
          max={30}
          step={1}
          value={trail.offsetX}
          onChange={(e) => patch({ offsetX: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Emit offset Y"
        value={`${trail.offsetY > 0 ? '+' : ''}${trail.offsetY} px`}
        help="Vertical offset. Positive = emit below the cursor anchor."
      >
        <input
          type="range"
          min={-30}
          max={30}
          step={1}
          value={trail.offsetY}
          onChange={(e) => patch({ offsetY: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Decay time"
        value={`${trail.lifetimeSeconds.toFixed(2)} s`}
        help="Total particle lifetime from spawn to disappearance."
      >
        <input
          type="range"
          min={0.2}
          max={5}
          step={0.05}
          value={trail.lifetimeSeconds}
          onChange={(e) =>
            patch({ lifetimeSeconds: Number(e.target.value) })
          }
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Drift grace"
        value={`${trail.driftGraceSeconds.toFixed(2)} s`}
        help="Seconds a particle holds its spawn position before random drift kicks in."
      >
        <input
          type="range"
          min={0}
          max={5}
          step={0.05}
          value={trail.driftGraceSeconds}
          onChange={(e) =>
            patch({ driftGraceSeconds: Number(e.target.value) })
          }
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Drift strength"
        value={`${trail.driftStrength} px/s`}
        help="Peak random-drift speed. Reached when a particle is at least 'Drift distance' pixels from the cursor."
      >
        <input
          type="range"
          min={0}
          max={400}
          step={5}
          value={trail.driftStrength}
          onChange={(e) => patch({ driftStrength: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Drift distance"
        value={`${trail.driftReferenceDistance} px`}
        help="Distance from cursor at which drift ease-in-outs to max. Short = particles disperse close to the cursor; long = only far trail particles scatter."
      >
        <input
          type="range"
          min={20}
          max={400}
          step={10}
          value={trail.driftReferenceDistance}
          onChange={(e) =>
            patch({ driftReferenceDistance: Number(e.target.value) })
          }
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Drift turn rate"
        value={trail.driftTurnRate.toFixed(2)}
        help="How fast each particle's drift direction changes over time. 0 = fixed direction; higher = swirling/meandering motion."
      >
        <input
          type="range"
          min={0}
          max={4}
          step={0.05}
          value={trail.driftTurnRate}
          onChange={(e) => patch({ driftTurnRate: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Drift flow scale"
        value={trail.driftFlowScale.toFixed(4)}
        help="Spatial coherence of the drift flow field. 0 = every particle wanders independently; higher = nearby particles move together like a breeze."
      >
        <input
          type="range"
          min={0}
          max={0.05}
          step={0.001}
          value={trail.driftFlowScale}
          onChange={(e) => patch({ driftFlowScale: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Particle count"
        value={`${trail.particleCount}`}
        help="Upper bound on live particles. Higher = denser trail at the cost of GPU work."
      >
        <input
          type="range"
          min={256}
          max={8192}
          step={256}
          value={trail.particleCount}
          onChange={(e) => patch({ particleCount: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <div className="flex items-center justify-between">
        <div>
          <div className="font-semibold">Emit when stationary</div>
          <div className="mt-0.5 text-[11px] opacity-60">
            Off = particles only emit while the cursor is moving.
          </div>
        </div>
        <label className="flex shrink-0 items-center gap-2">
          <input
            type="checkbox"
            checked={trail.emitWhenIdle}
            onChange={(e) => patch({ emitWhenIdle: e.target.checked })}
          />
          <span>{trail.emitWhenIdle ? 'On' : 'Off'}</span>
        </label>
      </div>

      <Field
        label="Fade-out grace"
        value={`${trail.fadeOutGraceSeconds.toFixed(2)} s`}
        help="How long a resting cursor waits before its remaining particles start globally fading."
      >
        <input
          type="range"
          min={0}
          max={2}
          step={0.05}
          value={trail.fadeOutGraceSeconds}
          onChange={(e) =>
            patch({ fadeOutGraceSeconds: Number(e.target.value) })
          }
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Fade-out duration"
        value={`${trail.fadeOutSeconds.toFixed(2)} s`}
        help="Time from start of fade to full invisibility, on top of the per-particle age fade."
      >
        <input
          type="range"
          min={0.05}
          max={2}
          step={0.05}
          value={trail.fadeOutSeconds}
          onChange={(e) => patch({ fadeOutSeconds: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Fade-out easing"
        help="ease-in holds alpha then drops off late. ease-out collapses the cluster fast, then tails off."
      >
        <select
          value={trail.fadeOutEasing}
          onChange={(e) =>
            patch({ fadeOutEasing: e.target.value as TrailFadeEasing })
          }
          className="mt-1 w-full rounded border border-zinc-300 bg-white px-2 py-1 text-[12px] dark:border-zinc-700 dark:bg-zinc-900"
        >
          <option value="linear">linear</option>
          <option value="ease-in">ease-in</option>
          <option value="ease-out">ease-out</option>
          <option value="ease-in-out">ease-in-out</option>
        </select>
      </Field>

      <Field
        label="Emit speed reference"
        value={`${trail.emitSpeedReferencePxPerSec} px/s`}
        help="Cursor speed at which emission reaches full rate. Lower = easier to fill the trail at low speeds."
      >
        <input
          type="range"
          min={50}
          max={2000}
          step={25}
          value={trail.emitSpeedReferencePxPerSec}
          onChange={(e) =>
            patch({ emitSpeedReferencePxPerSec: Number(e.target.value) })
          }
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Emit speed bias"
        value={trail.emitSpeedBias.toFixed(2)}
        help="Power curve on speed→emit. 1 = linear, >1 concentrates particles in the fast middle, <1 softens the bias."
      >
        <input
          type="range"
          min={0.25}
          max={4}
          step={0.05}
          value={trail.emitSpeedBias}
          onChange={(e) => patch({ emitSpeedBias: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Emits per page"
        value={`${trail.emitsPerFrame}`}
        help="Upper bound on particles spawned per cursor per page. Raise for denser trails; pair with a larger particle count so the ring buffer doesn't recycle too fast."
      >
        <input
          type="range"
          min={2}
          max={16}
          step={1}
          value={trail.emitsPerFrame}
          onChange={(e) => patch({ emitsPerFrame: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>
    </div>
  )
}

function TuningControls({
  splineViz,
  onSplineVizChange,
  tuning,
  onTuningChange,
  onTuningReset,
}: {
  splineViz: boolean
  onSplineVizChange: (on: boolean) => void
  tuning: CursorTuningParams
  onTuningChange: (next: CursorTuningParams) => void
  onTuningReset: () => void
}) {
  const patch = (p: Partial<CursorTuningParams>) =>
    onTuningChange({ ...tuning, ...p })

  return (
    <div className="flex flex-col gap-4 p-4 text-[12px]">
      <Header title="Director tuning" onReset={onTuningReset} />

      <Field
        label="Base speed"
        value={`${tuning.baseSpeedPxS} px/s`}
        help="Arc-length travel rate before distance scaling."
      >
        <input
          type="range"
          min={50}
          max={2000}
          step={10}
          value={tuning.baseSpeedPxS}
          onChange={(e) => patch({ baseSpeedPxS: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Distance scaling"
        value={tuning.distanceScaling.toFixed(2)}
        help="1 = constant speed (travel time grows with distance). 0 = constant duration regardless of distance, so short hops no longer teleport."
      >
        <input
          type="range"
          min={0}
          max={1}
          step={0.01}
          value={tuning.distanceScaling}
          onChange={(e) => patch({ distanceScaling: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Easing"
        help="Time-axis curve applied across each spline. Legacy default was easeInOutCubic; linear reproduces the constant-speed behavior."
      >
        <EasingPicker
          value={tuning.easing}
          onChange={(next) => patch({ easing: next })}
        />
      </Field>

      <Field
        label="Sync cap"
        value={`${tuning.syncCapMs} ms`}
        help="Upper bound for move-then-act. If the cursor arrives sooner, the mutation fires immediately; otherwise it fires at this cap."
      >
        <input
          type="range"
          min={0}
          max={1000}
          step={10}
          value={tuning.syncCapMs}
          onChange={(e) => patch({ syncCapMs: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Commit hold"
        value={`${tuning.commitHoldMs} ms`}
        help="Duration of the 'committing' phase (ripple hold) at commit waypoints."
      >
        <input
          type="range"
          min={0}
          max={1000}
          step={10}
          value={tuning.commitHoldMs}
          onChange={(e) => patch({ commitHoldMs: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <Field
        label="Dwell"
        value={`${tuning.commitDwellMs} ms`}
        help="Pause at non-commit waypoints that request a dwell."
      >
        <input
          type="range"
          min={0}
          max={1000}
          step={10}
          value={tuning.commitDwellMs}
          onChange={(e) => patch({ commitDwellMs: Number(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </Field>

      <hr className="border-[var(--surface-popover-border)]" />

      <div className="flex items-center justify-between">
        <div>
          <div className="font-semibold">Visualize cursor splines</div>
          <div className="mt-0.5 text-[11px] opacity-60">
            Overlays the active spline on the real canvas behind each agent
            cursor (separate from the playground above).
          </div>
        </div>
        <label className="flex shrink-0 items-center gap-2">
          <input
            type="checkbox"
            checked={splineViz}
            onChange={(e) => onSplineVizChange(e.target.checked)}
          />
          <span>{splineViz ? 'On' : 'Off'}</span>
        </label>
      </div>
    </div>
  )
}

function Header({ title, onReset }: { title: string; onReset: () => void }) {
  return (
    <div className="flex items-center justify-between">
      <div className="text-[11px] font-semibold uppercase tracking-wider opacity-60">
        {title}
      </div>
      <button
        type="button"
        onClick={onReset}
        className="rounded border border-zinc-300 px-2 py-0.5 text-[11px] hover:bg-zinc-100 dark:border-zinc-700 dark:hover:bg-zinc-800"
      >
        Reset
      </button>
    </div>
  )
}

function EasingPicker({
  value,
  onChange,
}: {
  value: EasingSpec
  onChange: (next: EasingSpec) => void
}) {
  const selectValue: EasingPreset | 'custom' =
    value.kind === 'custom' ? 'custom' : value.name

  const onSelect = (next: string) => {
    if (next === 'custom') {
      onChange({ kind: 'custom', x1: 0.4, y1: 0, x2: 0.2, y2: 1 })
    } else {
      onChange({ kind: 'preset', name: next as EasingPreset })
    }
  }

  return (
    <>
      <select
        value={selectValue}
        onChange={(e) => onSelect(e.target.value)}
        className="mt-1 w-full rounded border border-zinc-300 bg-white px-2 py-1 text-[12px] dark:border-zinc-700 dark:bg-zinc-900"
      >
        {EASING_PRESETS.map((preset) => (
          <option key={preset} value={preset}>
            {preset}
          </option>
        ))}
        <option value="custom">custom cubic-bezier</option>
      </select>
      {value.kind === 'custom' ? (
        <div className="mt-2 grid grid-cols-2 gap-2">
          {(['x1', 'y1', 'x2', 'y2'] as const).map((key) => (
            <div key={key}>
              <label className="text-[10px] uppercase tracking-wider opacity-60">
                {key}
              </label>
              <input
                type="number"
                step={0.05}
                value={value[key]}
                onChange={(e) =>
                  onChange({ ...value, [key]: Number(e.target.value) })
                }
                className="mt-0.5 w-full rounded border border-zinc-300 bg-white px-1 py-0.5 text-[12px] tabular-nums dark:border-zinc-700 dark:bg-zinc-900"
              />
            </div>
          ))}
        </div>
      ) : null}
    </>
  )
}

function Field({
  label,
  value,
  help,
  children,
}: {
  label: string
  value?: string
  help?: string
  children: ReactNode
}) {
  return (
    <div>
      <div className="flex items-baseline justify-between">
        <label className="text-[11px] font-medium opacity-70">{label}</label>
        {value !== undefined ? (
          <span className="text-[11px] tabular-nums opacity-60">{value}</span>
        ) : null}
      </div>
      {children}
      {help ? (
        <div className="mt-1 text-[10px] leading-snug opacity-55">{help}</div>
      ) : null}
    </div>
  )
}
