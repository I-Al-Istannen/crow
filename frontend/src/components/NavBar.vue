<template>
  <nav class="px-2 py-3 border-b border-border bg-background">
    <div class="container flex items-center justify-between max-w-7xl">
      <div class="flex space-x-6">
        <div class="flex items-center justify-center">
          <img src="/src/crow1337.svg" alt="logo" class="h-[24px]" />
          <span class="ml-2 font-semibold max-md:hidden">crow</span>
        </div>

        <NavigationMenu v-if="accountReady">
          <NavigationMenuList>
            <NavigationMenuItem v-for="route in routes" :key="route.title">
              <router-link :to="route.route">
                <NavigationMenuLink
                  :class="[
                    routerLinkClasses,
                    route.route.name == currentRoute.name ? 'bg-accent' : 'bg-background',
                  ]"
                >
                  {{ route.title }}
                </NavigationMenuLink>
              </router-link>
            </NavigationMenuItem>
          </NavigationMenuList>
        </NavigationMenu>
      </div>
      <div v-if="accountReady">
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button variant="outline" size="icon">
              <User></User>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuLabel class="flex flex-col space-y-1">
              {{ userName }}
            </DropdownMenuLabel>
            <DropdownMenuSeparator v-if="user && user.team" />
            <DropdownMenuItem as-child v-if="user && user.team">
              <router-link
                :to="{ name: 'team-info', params: { teamId: user.team } }"
                class="cursor-pointer"
              >
                Team
              </router-link>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem>Logout</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  </nav>
</template>

<script setup lang="ts">
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Home, User } from 'lucide-vue-next'
import {
  NavigationMenu,
  NavigationMenuItem,
  NavigationMenuLink,
  NavigationMenuList,
} from '@/components/ui/navigation-menu'
import { Button } from '@/components/ui/button'
import { computed } from 'vue'
import router from '@/router'
import { storeToRefs } from 'pinia'
import { useRoute } from 'vue-router'
import { useUserStore } from '@/stores/user.ts'

const routerLinkClasses =
  'group inline-flex h-9 w-max items-center justify-center rounded-md px-4 py-2' +
  ' text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground' +
  ' focus:bg-accent focus:text-accent-foreground focus:outline-none disabled:pointer-events-none' +
  ' disabled:opacity-50 data-[active]:bg-accent'

const currentRoute = useRoute()
const { user, accountReady } = storeToRefs(useUserStore())
const userName = computed(() => user.value?.displayName)

const routes = computed(() =>
  router
    .getRoutes()
    .filter((route) => !route.meta?.hidden)
    .map((route) => ({
      route: route,
      title: (route.meta?.name || route.name) as string,
      icon: route.meta?.icon || Home,
    })),
)
</script>
