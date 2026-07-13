import { useEffect, useRef, useState } from 'react'
import type {
  DevtoolsPanelSelectionSummary,
  InspectNodeDetail,
} from '../../shared/types'
import { rightDetailsPanelApi } from './rightDetailsPanelApi'

export function useElementCommentDraft({
  activeDetail,
  selection,
}: {
  activeDetail?: InspectNodeDetail
  selection?: DevtoolsPanelSelectionSummary
}) {
  const [elementCommentText, setElementCommentText] = useState('')
  const commentInputRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    setElementCommentText('')
  }, [activeDetail?.nodeId])

  const submitElementComment = () => {
    if (!activeDetail) return
    const text = elementCommentText.trim()
    if (!text) return
    rightDetailsPanelApi.createAnnotation({
      anchor: {
        type: 'element',
        pageId: activeDetail.pageId,
        selector: activeDetail.fullPath || activeDetail.elementPath,
        elementPath: activeDetail.elementPath,
        boundingBox: activeDetail.boundingBox,
      },
      author: 'user',
      text,
      metadata: {
        pageName: selection?.viewportLabel,
        inspectContext: {
          pageId: activeDetail.pageId,
          nodeId: activeDetail.nodeId,
          id: activeDetail.id,
          timestamp: activeDetail.timestamp,
          tagName: activeDetail.tagName,
          name: activeDetail.name,
          role: activeDetail.role,
          elementPath: activeDetail.elementPath,
          fullPath: activeDetail.fullPath,
          cssClasses: activeDetail.cssClasses,
          textPreview: activeDetail.textPreview,
          nearbyText: activeDetail.nearbyText,
          nearbyElements: activeDetail.nearbyElements,
          accessibility: activeDetail.accessibility,
          attributes: activeDetail.attributes,
          computedStyles: activeDetail.computedStyles,
          boundingBox: activeDetail.boundingBox,
          position: activeDetail.position,
          sourceLocation: activeDetail.sourceLocation,
        },
      },
    })
    setElementCommentText('')
  }

  return {
    commentInputRef,
    elementCommentText,
    hasElementComment: elementCommentText.trim().length > 0,
    setElementCommentText,
    submitElementComment,
  }
}
