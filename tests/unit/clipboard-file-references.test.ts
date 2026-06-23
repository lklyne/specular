import { describe, expect, it } from 'vitest'
import { filePathsFromClipboardReferences } from '../../src/main/clipboard-file-references'

describe('filePathsFromClipboardReferences', () => {
  it('extracts file URLs', () => {
    expect(
      filePathsFromClipboardReferences(['file:///Users/example/notes/test.md']),
    ).toEqual(['/Users/example/notes/test.md'])
  })

  it('extracts absolute paths', () => {
    expect(
      filePathsFromClipboardReferences(['/Users/example/site/index.html']),
    ).toEqual(['/Users/example/site/index.html'])
  })

  it('handles text/uri-list comments and gnome copy headers', () => {
    expect(
      filePathsFromClipboardReferences([
        'copy\n# copied files\nfile:///Users/example/a.md\nfile:///Users/example/b.html',
      ]),
    ).toEqual(['/Users/example/a.md', '/Users/example/b.html'])
  })

  it('dedupes repeated references', () => {
    expect(
      filePathsFromClipboardReferences([
        'file:///Users/example/a.md',
        '/Users/example/a.md',
      ]),
    ).toEqual(['/Users/example/a.md'])
  })

  it('ignores bare filenames because they cannot locate the file', () => {
    expect(filePathsFromClipboardReferences(['index.html'])).toEqual([])
  })
})
