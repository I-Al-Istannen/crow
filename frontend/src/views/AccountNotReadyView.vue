<template>
  <PageContainer v-if="loggedIn">
    <Card>
      <CardHeader>
        <CardTitle>Hey there, traveller</CardTitle>
        <CardDescription v-if="!loggedIn">Sadly, crow requires authentication.</CardDescription>
        <CardDescription v-else>You need to be part of a team</CardDescription>
      </CardHeader>
      <CardContent>
        <a :href="loginUrl" v-if="!loggedIn">
          <Button v-if="!loggedIn">Log in</Button>
        </a>
        <div v-else>
          It seems like you are not yet part of a team :)
          <br />
          You will be assigned by the course administrators. If you believe this is an error, feel
          encouraged to report it!
        </div>
      </CardContent>
    </Card>
  </PageContainer>
  <PageContainer v-else class="h-[calc(100dvh-49px)] bg-red-50" ref="crowContainer">
    <img
      v-for="crow in crows"
      :key="crow.id"
      src="/src/crow1337.svg"
      alt="a crow"
      class="h-[40px] w-[40px] relative border-black border"
      :class="{ '-scale-x-100': getDirection(crow) === 'left' }"
      :style="{
        top: `${crow.position.y}px`,
        left: `${crow.position.x}px`,
      }"
    />
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { computed, ref, watch } from 'vue'
import { BACKEND_URL } from '@/data/fetching.ts'
import { Button } from '@/components/ui/button'
import PageContainer from '@/components/PageContainer.vue'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user.ts'

const { loggedIn } = storeToRefs(useUserStore())
const crows = ref<Crow[]>([])
const lastAnimationTime = ref<number>(0)
const crowContainer = ref<InstanceType<typeof PageContainer> | null>(null)

const loginUrl = computed(() => BACKEND_URL + '/login')

watch(
  loggedIn,
  (loggedIn) => {
    if (!loggedIn) {
      requestAnimationFrame(animationLoop)
    }
  },
  { immediate: true },
)

type Crow = {
  id: string
  bounds: {
    width: number
    height: number
  }
  position: {
    x: number
    y: number
  }
  velocity: {
    dx: number
    dy: number
    curveTime: number
    slowingDown: boolean
    speedFactor: number
    curveStep: number
  }
}

type Bounds = {
  width: number
  height: number
}

function getDirection(crow: Crow): 'left' | 'right' {
  return crow.velocity.dx >= 0 ? 'right' : 'left'
}

function easeInSine(x: number): number {
  return 1 - Math.cos((x * Math.PI) / 2)
}

function moveCrow(crow: Crow, bounds: Bounds) {
  if (crow.position.x < 0 || (crow.position.x + crow.bounds.width) >= bounds.width) {
    console.log('hit wall')
    console.log(crow.position.x, crow.position.x + crow.bounds.width, bounds.width)
    crow.velocity.dx *= -1
  }
  if (crow.position.y < 0 || (crow.position.y + crow.bounds.height) >= bounds.height) {
    console.log('hit wall')
    console.log(crow.position.y, crow.position.y + crow.bounds.height, bounds.height)
    crow.velocity.dy *= -1
  }

  crow.position.x += crow.velocity.dx
  crow.position.y += crow.velocity.dy

  crow.velocity.curveTime += crow.velocity.curveStep
  if (crow.velocity.curveTime > 1) {
    crow.velocity.curveTime = crow.velocity.curveStep
    crow.velocity.dx = Math.random() * 2 - 1
    crow.velocity.dy = Math.random() * 2 - 1
    crow.velocity.slowingDown = !crow.velocity.slowingDown
  }

  const easeFactor = crow.velocity.slowingDown
    ? 1 - easeInSine(crow.velocity.curveTime)
    : easeInSine(crow.velocity.curveTime)

  crow.velocity.dx = crow.velocity.speedFactor * Math.sign(crow.velocity.dx) * easeFactor
  crow.velocity.dy = crow.velocity.speedFactor * Math.sign(crow.velocity.dy) * easeFactor
}

function spawnCrow(bounds: Bounds) {
  return {
    id: 'crow' + Math.random().toString(16).slice(2),
    bounds: {
      width: 40,
      height: 40,
    },
    position: {
      x: Math.random() * bounds.width,
      y: Math.random() * bounds.height,
    },
    velocity: {
      dx: Math.random() * 2 - 1,
      dy: Math.random() * 2 - 1,
      curveTime: Math.random() * Math.PI * 2,
      slowingDown: Math.random() > 0.5,
      speedFactor: Math.random() * 5,
      curveStep: Math.random() * 0.01,
    },
  }
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

  if (crows.value.length === 0) {
    crows.value.push(spawnCrow(bounds))
  }

  for (const crow of crows.value) {
    moveCrow(crow, bounds)
  }
}
</script>
