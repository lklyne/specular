const size = +SIZE
const c = document.querySelector('canvas')
const el = c.parentElement.parentElement.parentElement
el.style.width = size + 'px'; el.style.height = size + 'px'
await window.__gpu.waitReal(30)
return [c.width, c.height]
