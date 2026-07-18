import { afterEach, describe, it, expect } from 'vitest'
import {
  buildShapeEntitySceneEntity,
  clearShapeEntities,
  createShapeEntity,
  DEFAULT_SHAPE_HEIGHT,
  DEFAULT_SHAPE_WIDTH,
  deleteShapeEntity,
  persistShapeEntity,
  shapeEntities,
  updateShapeEntity,
} from '../../src/main/runtime/shape-entity-state'

afterEach(() => {
  clearShapeEntities()
})

describe('shape-entity-state', () => {
  it('creates a rectangle with default size when nothing is supplied', () => {
    const entity = createShapeEntity({ canvasX: 10, canvasY: 20 })
    expect(entity.shapeKind).toBe('rectangle')
    expect(entity.width).toBe(DEFAULT_SHAPE_WIDTH)
    expect(entity.height).toBe(DEFAULT_SHAPE_HEIGHT)
    expect(entity.text).toBe('')
    expect(shapeEntities).toHaveLength(1)
  })

  it('honors explicit shapeKind, size, and text', () => {
    const entity = createShapeEntity({
      canvasX: 0,
      canvasY: 0,
      shapeKind: 'ellipse',
      width: 80,
      height: 40,
      text: 'hello',
      color: '2',
      strokeWidth: 3,
    })
    expect(entity.shapeKind).toBe('ellipse')
    expect(entity.width).toBe(80)
    expect(entity.height).toBe(40)
    expect(entity.text).toBe('hello')
    expect(entity.color).toBe('2')
    expect(entity.strokeWidth).toBe(3)
  })

  it('updates fields including shapeKind and text', () => {
    const entity = createShapeEntity({ canvasX: 0, canvasY: 0 })
    updateShapeEntity(entity.id, { shapeKind: 'diamond', text: 'go' })
    expect(entity.shapeKind).toBe('diamond')
    expect(entity.text).toBe('go')
  })

  it('deletes by id', () => {
    const entity = createShapeEntity({ canvasX: 0, canvasY: 0 })
    expect(deleteShapeEntity(entity.id)).toBe(true)
    expect(shapeEntities).toHaveLength(0)
  })

  it('builds a scene entity with screen-space coords', () => {
    const entity = createShapeEntity({
      canvasX: 100,
      canvasY: 200,
      width: 50,
      height: 30,
      shapeKind: 'rectangle',
    })
    const scene = buildShapeEntitySceneEntity(entity, 2, { x: 5, y: 7 }, { x: 10, y: 20 })
    expect(scene.kind).toBe('shape')
    // canvasOrigin + canvasX*zoom + pan
    expect(scene.screenX).toBe(10 + 100 * 2 + 5)
    expect(scene.screenY).toBe(20 + 200 * 2 + 7)
    expect(scene.screenWidth).toBe(100)
    expect(scene.screenHeight).toBe(60)
  })

  it('persists and restores all fields', () => {
    const entity = createShapeEntity({
      canvasX: 1,
      canvasY: 2,
      shapeKind: 'diamond',
      text: 'go',
      color: '3',
      strokeWidth: 4,
      width: 80,
      height: 90,
    })
    const persisted = persistShapeEntity(entity)
    expect(persisted).toMatchObject({
      kind: 'shape',
      id: entity.id,
      shapeKind: 'diamond',
      text: 'go',
      color: '3',
      strokeWidth: 4,
      width: 80,
      height: 90,
      canvasX: 1,
      canvasY: 2,
    })
  })

  it('carries fill and label styling through create, update, scene, and persist', () => {
    const entity = createShapeEntity({
      canvasX: 0,
      canvasY: 0,
      fillStyle: 'none',
      textSize: 56,
      textAlign: 'left',
      textVerticalAlign: 'top',
    })
    expect(entity.fillStyle).toBe('none')
    expect(entity.textSize).toBe(56)
    updateShapeEntity(entity.id, {
      fillStyle: 'solid',
      textSize: 32,
      textAlign: 'right',
      textVerticalAlign: 'bottom',
    })
    expect(entity.fillStyle).toBe('solid')
    expect(entity.textSize).toBe(32)
    const scene = buildShapeEntitySceneEntity(entity, 1, { x: 0, y: 0 }, { x: 0, y: 0 })
    expect(scene).toMatchObject({
      fillStyle: 'solid',
      textSize: 32,
      textAlign: 'right',
      textVerticalAlign: 'bottom',
    })
    expect(persistShapeEntity(entity)).toMatchObject({
      fillStyle: 'solid',
      textSize: 32,
      textAlign: 'right',
      textVerticalAlign: 'bottom',
    })
  })
})
