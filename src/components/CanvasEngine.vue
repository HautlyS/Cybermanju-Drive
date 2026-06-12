<template>
  <canvas ref="canvasRef" class="canvas-engine" />
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'

const props = withDefaults(defineProps<{
  enabled?: boolean
  mode?: number
}>(), {
  enabled: true,
  mode: 0,
})

const canvasRef = ref<HTMLCanvasElement | null>(null)
let animId = 0
let ctx: CanvasRenderingContext2D | null = null
let w = 0, h = 0
let t = 0

let verts: Float64Array
let edges: Int32Array
let rotX = 0, rotY = 0, rotZ = 0

let lissajousPhase = 0
let particles: { x: number; y: number; vx: number; vy: number; life: number }[] = []
let scanlineOffset = 0

function resize() {
  const c = canvasRef.value
  if (!c) return
  const dpr = window.devicePixelRatio || 1
  w = window.innerWidth
  h = window.innerHeight
  c.width = w * dpr
  c.height = h * dpr
  c.style.width = w + 'px'
  c.style.height = h + 'px'
  if (ctx) ctx.scale(dpr, dpr)
}

function initWireframe() {
  const s = 60
  verts = new Float64Array([
    -s,-s,-s,  s,-s,-s,  s,s,-s,  -s,s,-s,
    -s,-s, s,  s,-s, s,  s,s, s,  -s,s, s,
  ])
  edges = new Int32Array([
    0,1, 1,2, 2,3, 3,0,
    4,5, 5,6, 6,7, 7,4,
    0,4, 1,5, 2,6, 3,7,
  ])
}

function initParticles() {
  particles = []
  for (let i = 0; i < 80; i++) {
    particles.push({
      x: Math.random() * w,
      y: Math.random() * h,
      vx: (Math.random() - 0.5) * 2,
      vy: (Math.random() - 0.5) * 2 - 1,
      life: Math.random(),
    })
  }
}

function project(x: number, y: number, z: number): [number, number] {
  const fov = 300
  const cx = w / 2
  const cy = h / 2
  const scale = fov / (fov + z)
  return [x * scale + cx, -y * scale + cy]
}

function rotate3d(x: number, y: number, z: number): [number, number, number] {
  let rx = x, ry = y, rz = z
  let t = ry
  ry = t * Math.cos(rotX) - rz * Math.sin(rotX)
  rz = t * Math.sin(rotX) + rz * Math.cos(rotX)
  t = rx
  rx = t * Math.cos(rotY) + rz * Math.sin(rotY)
  rz = -t * Math.sin(rotY) + rz * Math.cos(rotY)
  t = rx
  rx = t * Math.cos(rotZ) - ry * Math.sin(rotZ)
  ry = t * Math.sin(rotZ) + ry * Math.cos(rotZ)
  return [rx, ry, rz]
}

function drawWireframe() {
  if (!ctx || !verts || !edges) return
  const numVerts = verts.length / 3
  const projected: [number, number][] = []
  for (let i = 0; i < numVerts; i++) {
    const [rx, ry, rz] = rotate3d(verts[i*3], verts[i*3+1], verts[i*3+2])
    projected.push(project(rx, ry, rz))
  }
  ctx.strokeStyle = '#FFFFFF'
  ctx.lineWidth = 1.5
  for (let i = 0; i < edges.length; i += 2) {
    const p1 = projected[edges[i]]
    const p2 = projected[edges[i+1]]
    ctx.beginPath()
    ctx.moveTo(p1[0], p1[1])
    ctx.lineTo(p2[0], p2[1])
    ctx.stroke()
  }
  for (const p of projected) {
    ctx.fillStyle = '#FFFFFF'
    ctx.fillRect(p[0] - 2, p[1] - 2, 4, 4)
  }
}

function drawLissajous() {
  if (!ctx) return
  ctx.strokeStyle = '#FFFFFF'
  ctx.lineWidth = 1
  const cx = w / 2
  const cy = h / 2
  const r = Math.min(w, h) * 0.3
  ctx.beginPath()
  const steps = 200
  const a = 3, b = 4
  for (let i = 0; i <= steps; i++) {
    const th = (i / steps) * Math.PI * 2
    const x = cx + r * Math.sin(a * th + lissajousPhase * 0.3)
    const y = cy + r * Math.sin(b * th + lissajousPhase * 0.5)
    if (i === 0) ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  }
  ctx.stroke()
  const s2 = 5, b2 = 6
  ctx.beginPath()
  for (let i = 0; i <= steps; i++) {
    const th = (i / steps) * Math.PI * 2
    const x = cx + r * 0.6 * Math.sin(s2 * th + lissajousPhase * 0.2 + 1)
    const y = cy + r * 0.6 * Math.sin(b2 * th + lissajousPhase * 0.4 + 2)
    if (i === 0) ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  }
  ctx.stroke()
}

function drawParticles() {
  if (!ctx) return
  for (const p of particles) {
    p.x += p.vx
    p.y += p.vy
    p.life -= 0.005
    if (p.life <= 0 || p.x < 0 || p.x > w || p.y < 0 || p.y > h) {
      p.x = Math.random() * w
      p.y = h + 10
      p.vx = (Math.random() - 0.5) * 2
      p.vy = -(1 + Math.random() * 2)
      p.life = 0.5 + Math.random() * 0.5
    }
    const alpha = Math.max(0, p.life)
    ctx.fillStyle = `rgba(255, 255, 255, ${alpha})`
    ctx.fillRect(p.x, p.y, 3, 3)
  }
}

function drawOpArtRings() {
  if (!ctx) return
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.06)'
  ctx.lineWidth = 1
  const cx = w / 2
  const cy = h / 2
  const maxR = Math.sqrt(w * w + h * h) / 2 + 50
  const offset = (t * 0.3) % 40
  for (let r = offset; r < maxR; r += 40) {
    ctx.beginPath()
    ctx.arc(cx, cy, r, 0, Math.PI * 2)
    ctx.stroke()
  }
}

function drawCheckerboard() {
  if (!ctx) return
  const size = 20
  const offX = (t * 0.5) % (size * 2)
  const offY = (t * 0.3) % (size * 2)
  ctx.fillStyle = 'rgba(255, 255, 255, 0.03)'
  for (let x = -size * 2 + offX; x < w + size * 2; x += size * 2) {
    for (let y = -size * 2 + offY; y < h + size * 2; y += size * 2) {
      ctx.fillRect(x, y, size, size)
      ctx.fillRect(x + size, y + size, size, size)
    }
  }
}

function drawScanlines() {
  if (!ctx) return
  scanlineOffset = (scanlineOffset + 2) % 4
  ctx.fillStyle = 'rgba(255, 255, 255, 0.015)'
  for (let y = scanlineOffset; y < h; y += 4) {
    ctx.fillRect(0, y, w, 1)
  }
}

function draw() {
  if (!ctx) return
  t++
  rotX = t * 0.008
  rotY = t * 0.012
  rotZ = t * 0.005
  lissajousPhase += 0.02

  ctx.clearRect(0, 0, w, h)

  drawOpArtRings()
  drawCheckerboard()

  const modeCycle = Math.floor(t / 300) % 3
  switch (modeCycle) {
    case 0:
      initWireframe()
      drawWireframe()
      break
    case 1:
      drawLissajous()
      drawParticles()
      break
    case 2:
      drawWireframe()
      drawParticles()
      break
  }

  drawScanlines()

  animId = requestAnimationFrame(draw)
}

function start() {
  if (animId) return
  resize()
  initParticles()
  initWireframe()
  draw()
}

function stop() {
  if (animId) {
    cancelAnimationFrame(animId)
    animId = 0
  }
}

onMounted(() => {
  const c = canvasRef.value
  if (c) {
    ctx = c.getContext('2d')
  }
  if (props.enabled) start()
  window.addEventListener('resize', resize)
})

onUnmounted(() => {
  stop()
  window.removeEventListener('resize', resize)
})

watch(() => props.enabled, (v) => {
  v ? start() : stop()
})
</script>

<style scoped>
.canvas-engine {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  z-index: 0;
  pointer-events: none;
}
</style>
