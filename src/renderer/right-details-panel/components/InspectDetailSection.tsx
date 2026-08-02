import type { InspectNodeDetail } from '../../../shared/types'
import {
  formatSourceLocation,
  parseEditedValue,
  valuePreview,
} from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'

export function InspectDetailSection({
  activeDetail,
  hoveredDetail,
  mutedClass,
  selectedDetail,
}: {
  activeDetail?: InspectNodeDetail
  hoveredDetail?: InspectNodeDetail
  mutedClass: string
  selectedDetail?: InspectNodeDetail
}) {
  const sourceLocationText = formatSourceLocation(activeDetail?.sourceLocation)

  return (
    <>
      {!activeDetail ? (
        <p className={`mt-2 text-[11px] leading-5 ${mutedClass}`}>No element captured yet.</p>
      ) : (
        <div className="mt-3 space-y-3">
          <div>
            <div className="text-[12px] font-medium">{activeDetail.name}</div>
            <div className={`mt-1 overflow-hidden text-ellipsis text-[11px] leading-5 ${mutedClass}`} style={{ display: '-webkit-box', WebkitLineClamp: 3, WebkitBoxOrient: 'vertical' }}>
              {activeDetail.elementPath}
            </div>
          </div>

          {sourceLocationText ? (
            <div className="grid grid-cols-[48px_minmax(0,1fr)] gap-2 text-[11px] leading-5">
              <div className={mutedClass}>Source</div>
              <div className="truncate font-mono" title={sourceLocationText}>
                {sourceLocationText}
              </div>
            </div>
          ) : null}

          <div className="grid grid-cols-2 gap-2 text-[11px] leading-5">
            <div className={mutedClass}>Tag {activeDetail.tagName}</div>
            <div className={mutedClass}>Node {activeDetail.nodeId}</div>
          </div>

          {activeDetail.boundingBox ? (
            <div className={`grid grid-cols-2 gap-2 text-[11px] leading-5 ${mutedClass}`}>
              <div>
                Bounds {activeDetail.boundingBox.width} x {activeDetail.boundingBox.height}
              </div>
              <div>
                Origin {activeDetail.boundingBox.x}, {activeDetail.boundingBox.y}
              </div>
            </div>
          ) : null}

          {activeDetail.props
            ? Object.entries(activeDetail.props).slice(0, 10).map(([key, value]) => (
                <div key={`p:${activeDetail.nodeId}:${key}`} className="text-[10px]">
                  <div className="mb-0.5 text-[var(--surface-panel-foreground-muted)]">{key}</div>
                  <input
                    className="w-full rounded border border-zinc-300 bg-white px-1 py-0.5 text-[10px] text-[var(--surface-panel-foreground)] outline-none focus:border-blue-400 dark:border-zinc-700 dark:bg-zinc-800"
                    defaultValue={valuePreview(value)}
                    onBlur={(event) =>
                      rightDetailsPanelApi.editComponentProp(activeDetail.pageId, {
                        componentId: activeDetail.nodeId,
                        propPath: [key],
                        value: parseEditedValue(event.currentTarget.value, value),
                      })
                    }
                  />
                </div>
              ))
            : null}

          {activeDetail.tokens
            ? Object.entries(activeDetail.tokens).slice(0, 10).map(([key, value]) => (
                <div key={`t:${activeDetail.nodeId}:${key}`} className="text-[10px]">
                  <div className="mb-0.5 text-[var(--surface-panel-foreground-muted)]">{key}</div>
                  <input
                    className="w-full rounded border border-zinc-300 bg-white px-1 py-0.5 text-[10px] text-[var(--surface-panel-foreground)] outline-none focus:border-blue-400 dark:border-zinc-700 dark:bg-zinc-800"
                    defaultValue={value || ''}
                    onBlur={(event) =>
                      rightDetailsPanelApi.editComponentToken(activeDetail.pageId, {
                        componentId: activeDetail.nodeId,
                        token: key,
                        value: event.currentTarget.value,
                      })
                    }
                  />
                </div>
              ))
            : null}

          {activeDetail.dsVariants && Object.keys(activeDetail.dsVariants).length > 0 ? (
            <div>
              <div className={`mb-1 text-[10px] font-semibold ${mutedClass}`}>Variants</div>
              <div className="flex flex-wrap gap-1">
                {Object.entries(activeDetail.dsVariants).map(([key, value]) => (
                  <span
                    key={`v:${activeDetail.nodeId}:${key}`}
                    className="rounded bg-violet-100 px-1.5 py-0.5 text-[10px] text-violet-800 dark:bg-violet-900/40 dark:text-violet-300"
                  >
                    {key}: {value}
                  </span>
                ))}
              </div>
            </div>
          ) : null}

          {activeDetail.dsPropSignature?.length ? (
            <div>
              <div className={`mb-1 text-[10px] font-semibold ${mutedClass}`}>DS Props</div>
              {activeDetail.dsPropSignature.map((prop) => (
                <div key={`ds:${activeDetail.nodeId}:${prop.name}`} className="text-[10px] leading-5">
                  <span className="font-medium">{prop.name}</span>
                  <span className={` ml-1 ${mutedClass}`}>{prop.type}</span>
                  {prop.values?.length ? (
                    <span className={` ml-1 ${mutedClass}`}>[{prop.values.join(', ')}]</span>
                  ) : null}
                  {prop.defaultValue !== undefined ? (
                    <span className={` ml-1 ${mutedClass}`}>= {prop.defaultValue}</span>
                  ) : null}
                </div>
              ))}
            </div>
          ) : null}

          {selectedDetail && selectedDetail.nodeId !== activeDetail.nodeId ? (
            <div className={`text-[11px] ${mutedClass}`}>Pinned node: {selectedDetail.nodeId}</div>
          ) : null}
          {hoveredDetail && hoveredDetail.nodeId !== activeDetail.nodeId ? (
            <div className={`text-[11px] ${mutedClass}`}>Hover node: {hoveredDetail.nodeId}</div>
          ) : null}
        </div>
      )}
    </>
  )
}
