import { Code, File, FileText, Image, Video, type LucideIcon } from 'lucide-react'
import {
  HTML_EXTENSIONS,
  IMAGE_EXTENSIONS,
  MARKDOWN_EXTENSIONS,
  VIDEO_EXTENSIONS,
} from '../../shared/file-extensions'

/** Pick a lucide icon based on file extension. Used in the sidebar tree
 *  and the canvas chrome header so a note shows a text icon, an image
 *  shows a picture icon, etc. */
export function iconForFilePath(filePath: string): LucideIcon {
  if (MARKDOWN_EXTENSIONS.test(filePath)) return FileText
  if (IMAGE_EXTENSIONS.test(filePath)) return Image
  if (VIDEO_EXTENSIONS.test(filePath)) return Video
  if (HTML_EXTENSIONS.test(filePath)) return Code
  return File
}
