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
  <PageContainer
    v-else
    class="h-[calc(100dvh-49px)] overflow-clip relative max-w-full"
    ref="crowContainer"
    @mouseleave="mousePos = { x: -100, y: -100 }"
    @mousemove="mousePos = { x: $event.offsetX, y: $event.offsetY }"
  >
    <a :href="BACKEND_URL + '/login'">
      <Button class="absolute left-1/2 top-1/2 -translate-y-1/2 -translate-x-1/2">Log in</Button>
    </a>
    <img
      v-for="crow in crows"
      :key="crow.id"
      :id="crow.id"
      src="/src/crow1337.svg"
      alt="a crow"
      class="h-[25px] w-[25px] absolute pointer-events-none"
    />
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { computed, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { BACKEND_URL } from '@/data/fetching.ts'
import { Boid } from '@/lib/boids.ts'
import { Button } from '@/components/ui/button'
import PageContainer from '@/components/PageContainer.vue'
import { Vec2d } from '@/lib/maths.ts'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user.ts'

const { loggedIn } = storeToRefs(useUserStore())
const crows = shallowRef<Boid[]>([])
const lastAnimationTime = ref<number>(0)
const crowContainer = ref<InstanceType<typeof PageContainer> | null>(null)
const mousePos = ref<{ x: number; y: number }>({ x: -100, y: -100 })
const animationId = ref<number | null>(null)

const loginUrl = computed(() => BACKEND_URL + '/login')

watch(
  loggedIn,
  (loggedIn) => {
    if (!loggedIn) {
      animationId.value = requestAnimationFrame(animationLoop)
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  if (animationId.value) {
    cancelAnimationFrame(animationId.value)
  }
})

function spawnCrow(bounds: { width: number; height: number }): Boid {
  return new Boid(
    'crow' + Math.random().toString(16).slice(2),
    new Vec2d(Math.random() * bounds.width, Math.random() * bounds.height),
    new Vec2d(Math.random() * 2 - 1, Math.random() * 2 - 1),
    25,
    25,
  )
}

function maxBoidCount() {
  return Math.min(50, Math.max(window.screen.width, window.screen.height) / 15)
}

function renderCrow(crow: Boid) {
  const crowElement = document.getElementById(crow.id)
  if (!crowElement) {
    return
  }

  crowElement.style.transform = `rotate(${crow.velocity.angle(Vec2d.RIGHT)}rad)`
  crowElement.style.left = `${crow.pos.x}px`
  crowElement.style.top = `${crow.pos.y}px`
}

function animationLoop(time: number) {
  if (loggedIn.value) {
    return
  }
  requestAnimationFrame(animationLoop)

  const timeDelta = time - lastAnimationTime.value
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

  const bounds2d = new Vec2d(bounds.width, bounds.height)
  const mouse2d = mousePos.value ? new Vec2d(mousePos.value.x, mousePos.value.y) : Vec2d.ZERO

  for (const crow of crows.value) {
    crow.update(bounds2d, mouse2d, crows.value, timeDelta / 1000)
    renderCrow(crow)
  }
}
</script>
