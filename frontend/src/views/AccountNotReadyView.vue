<template>
  <PageContainer v-if="loggedIn">
    <Card v-if="isLoadingMyself || error">
      <CardHeader>
        <CardTitle>Hey there, traveller</CardTitle>
        <CardDescription>Loading your user information...</CardDescription>
      </CardHeader>
      <CardContent>
        <DataLoadingExplanation
          :is-loading="isLoadingMyself"
          :failure-count="failureCount"
          :failure-reason="failureReason"
        />
      </CardContent>
    </Card>
    <Card v-else>
      <CardHeader>
        <CardTitle>Hey there, traveller</CardTitle>
        <CardDescription>You need to be part of a team</CardDescription>
      </CardHeader>
      <CardContent>
        It seems like you are not yet part of a team :)
        <br />
        You will be assigned by the course administrators. If you believe this is an error, feel
        encouraged to report it!
      </CardContent>
    </Card>
  </PageContainer>
  <PageContainer
    v-else
    class="relative h-[calc(100dvh-49px)] max-w-full overflow-clip"
    ref="crowContainer"
    @mouseleave="mousePos = { x: -100, y: -100 }"
    @mousemove="mousePos = { x: $event.offsetX, y: $event.offsetY }"
  >
    <a :href="BACKEND_URL + '/login'" @click="saveFromUrl">
      <Button class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">Log in</Button>
    </a>
    <img
      v-for="crow in crows"
      :key="crow.id"
      :id="crow.id"
      src="/src/crow1337.svg"
      alt="a crow"
      class="pointer-events-none absolute h-[25px] w-[25px]"
    />
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ref, shallowRef, watch } from 'vue'
import { BACKEND_URL } from '@/data/fetching.ts'
import { Button } from '@/components/ui/button'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import { PRE_LOGIN_URL_SESSION_STORAGE_KEY } from '@/router'
import PageContainer from '@/components/PageContainer.vue'
import { queryMyself } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { useRoute } from 'vue-router'
import { useUserStore } from '@/stores/user.ts'

const { loggedIn } = storeToRefs(useUserStore())
const crows = shallowRef<Crow[]>([])
const lastAnimationTime = ref<number>(0)
const crowContainer = ref<InstanceType<typeof PageContainer> | null>(null)
const mousePos = ref<{ x: number; y: number }>({ x: -100, y: -100 })

const currentRoute = useRoute()
const { isLoading: isLoadingMyself, error, failureCount, failureReason } = queryMyself()

watch(
  loggedIn,
  (loggedIn) => {
    if (!loggedIn) {
      requestAnimationFrame(animationLoop)
    }
  },
  { immediate: true },
)

const saveFromUrl = () => {
  sessionStorage.setItem(PRE_LOGIN_URL_SESSION_STORAGE_KEY, currentRoute.fullPath)
}

type Crow = {
  id: string
  currentDirection: 'left' | 'right'
  bounds: {
    width: number
    height: number
  }
  img: HTMLImageElement | null
  position: {
    x: number
    y: number
  }
  velocity: {
    dx: number
    dy: number
  }
}

type Bounds = {
  width: number
  height: number
}

function getDirection(crow: Crow): 'left' | 'right' {
  return crow.velocity.dx >= 0 ? 'right' : 'left'
}

function distSquared(a: { x: number; y: number }, b: { x: number; y: number }) {
  return (a.x - b.x) ** 2 + (a.y - b.y) ** 2
}

const VISUAL_RANGE = 200 ** 2
const MAX_SPEED = 10

function coherence(crow: Crow) {
  let centerX = 0
  let centerY = 0
  let neighbours = 0
  for (const other of crows.value) {
    if (other.id === crow.id) {
      continue
    }
    if (distSquared(crow.position, other.position) > VISUAL_RANGE) {
      continue
    }
    centerX += other.position.x
    centerY += other.position.y
    neighbours++
  }

  if (neighbours === 0) {
    return {
      dx: 0,
      dy: 0,
    }
  }

  // Rule 1: Coherence
  const moveForce = {
    dx: centerX / neighbours - crow.position.x,
    dy: centerY / neighbours - crow.position.y,
  }
  moveForce.dx /= 100
  moveForce.dy /= 100

  return moveForce
}

function separation(crow: Crow) {
  let separationX = 0
  let separationY = 0
  for (const other of crows.value) {
    if (other.id === crow.id) {
      continue
    }
    if (distSquared(crow.position, other.position) > 50 ** 2) {
      continue
    }

    separationX -= other.position.x - crow.position.x
    separationY -= other.position.y - crow.position.y
  }

  return {
    dx: separationX / 30,
    dy: separationY / 30,
  }
}

function alignment(crow: Crow) {
  let averageX = 0
  let averageY = 0
  let neighbours = 0
  for (const other of crows.value) {
    if (other.id === crow.id) {
      continue
    }
    if (distSquared(crow.position, other.position) > VISUAL_RANGE) {
      continue
    }
    averageX += other.velocity.dx
    averageY += other.velocity.dy
    neighbours++
  }
  averageX /= neighbours
  averageY /= neighbours

  if (neighbours === 0) {
    return {
      dx: 0,
      dy: 0,
    }
  }

  return {
    dx: (averageX - crow.velocity.dx) / 8,
    dy: (averageY - crow.velocity.dy) / 8,
  }
}

function repelMouse(crow: Crow) {
  const distance = distSquared(crow.position, mousePos.value)
  if (distance > 150 ** 2) {
    return {
      dx: 0,
      dy: 0,
    }
  }

  return {
    dx: (crow.position.x - mousePos.value.x) * 10,
    dy: (crow.position.y - mousePos.value.y) * 10,
  }
}

function moveCrow(crow: Crow, bounds: Bounds) {
  const coherenceForce = coherence(crow)
  const separationForce = separation(crow)
  const alignmentForce = alignment(crow)
  const mouseForce = repelMouse(crow)

  crow.velocity.dx += coherenceForce.dx + separationForce.dx + alignmentForce.dx + mouseForce.dx
  crow.velocity.dy += coherenceForce.dy + separationForce.dy + alignmentForce.dy + mouseForce.dy

  if (Math.abs(crow.velocity.dx) > MAX_SPEED) {
    crow.velocity.dx = (crow.velocity.dx / Math.abs(crow.velocity.dx)) * MAX_SPEED
  }
  if (Math.abs(crow.velocity.dy) > MAX_SPEED) {
    crow.velocity.dy = (crow.velocity.dy / Math.abs(crow.velocity.dy)) * MAX_SPEED
  }

  if (crow.position.x < 0) {
    crow.velocity.dx = MAX_SPEED / 2
  } else if (crow.position.x + crow.bounds.width >= bounds.width) {
    crow.velocity.dx = -MAX_SPEED / 2
  }
  if (crow.position.y < 0) {
    crow.velocity.dy = MAX_SPEED / 2
  } else if (crow.position.y + crow.bounds.height >= bounds.height) {
    crow.velocity.dy = -MAX_SPEED / 2
  }

  crow.position.x += crow.velocity.dx
  crow.position.y += crow.velocity.dy

  const crowElement = crow.img ? crow.img : document.getElementById(crow.id)!
  if (!crowElement) {
    return
  }

  const direction = getDirection(crow)
  if (direction != crow.currentDirection) {
    if (direction === 'left') {
      crowElement.style.transform = 'scaleX(-1)'
    } else {
      crowElement.style.transform = ''
    }
    crow.currentDirection = direction
  }

  crowElement.style.top = `${crow.position.y}px`
  crowElement.style.left = `${crow.position.x}px`
}

function spawnCrow(bounds: Bounds): Crow {
  return {
    id: 'crow' + Math.random().toString(16).slice(2),
    currentDirection: 'right',
    bounds: {
      width: 25,
      height: 25,
    },
    img: null,
    position: {
      x: Math.random() * bounds.width,
      y: Math.random() * bounds.height,
    },
    velocity: {
      dx: Math.random() * 2 - 1,
      dy: Math.random() * 2 - 1,
    },
  }
}

function maxBoidCount() {
  return Math.min(200, Math.max(window.screen.width, window.screen.height) / 15)
}

function animationLoop(time: number) {
  const MAX_FPS = 60

  if (loggedIn.value) {
    return
  }
  requestAnimationFrame(animationLoop)

  const timeDelta = time - lastAnimationTime.value
  if (timeDelta < 1000 / MAX_FPS) {
    return
  }
  lastAnimationTime.value = time

  const bounds = crowContainer.value?.$el.getBoundingClientRect()
  if (!bounds) {
    return
  }

  if (crows.value.length < maxBoidCount()) {
    crows.value.push(spawnCrow(bounds))
    crows.value = crows.value.slice()
  }
  if (crows.value.length > maxBoidCount()) {
    crows.value.pop()
    crows.value = crows.value.slice()
  }

  for (const crow of crows.value) {
    moveCrow(crow, bounds)
  }
}
</script>
