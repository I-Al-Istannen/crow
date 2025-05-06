<template>
  <nav class="px-2 py-3 border-b border-border bg-background">
    <div class="container flex items-center justify-between max-w-7xl max-sm:px-0">
      <div class="flex md:space-x-6 flex-grow">
        <router-link :to="{ name: 'home' }" class="flex items-center flex-shrink-0">
          <div class="flex items-center justify-center flex-shrink-0">
            <img src="/src/crow1337.svg" alt="logo" class="h-[24px] flex-shrink-0" />
            <span class="ml-2 font-semibold max-md:hidden">crow</span>
          </div>
        </router-link>

        <NavigationMenu v-if="accountReady" class="w-full max-sm:max-w-full">
          <NavigationMenuList class="w-full">
            <NavigationMenuItem v-for="route in routes" :key="route.title">
              <router-link :to="route.route">
                <NavigationMenuLink
                  :class="[
                    routerLinkClasses,
                    route.route.name == currentRoute.name ? 'bg-accent' : 'bg-background',
                  ]"
                  class="max-sm:px-2"
                >
                  {{ route.title }}
                </NavigationMenuLink>
              </router-link>
            </NavigationMenuItem>
          </NavigationMenuList>
        </NavigationMenu>
      </div>
      <div v-if="loggedIn">
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <div :class="clsx(isAdmin && 'p-[2px] bg-gradient-primary rounded-md')">
              <Button variant="outline" size="icon">
                <User />
              </Button>
            </div>
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
                <span class="flex-grow" />
                <LucideUsers />
              </router-link>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem @click="doLogout" class="cursor-pointer">
              Logout
              <span class="flex-grow" />
              <LucideLogOut />
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <a href="https://github.com/I-Al-Istannen/crow" target="_blank" rel="noopener">
              <DropdownMenuItem class="cursor-pointer">
                GitHub
                <span class="flex-grow" />
                <LucideExternalLink />
              </DropdownMenuItem>
            </a>
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
import { Home, LucideExternalLink, LucideLogOut, LucideUsers, User } from 'lucide-vue-next'
import {
  NavigationMenu,
  NavigationMenuItem,
  NavigationMenuLink,
  NavigationMenuList,
} from '@/components/ui/navigation-menu'
import { Button } from '@/components/ui/button'
import { clsx } from 'clsx'
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
const { accountReady, isAdmin, loggedIn, user } = storeToRefs(useUserStore())
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

const doLogout = async () => {
  useUserStore().logOut()
  await router.push('/')
}
</script>
