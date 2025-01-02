<template>
  <nav class="px-2 py-3 border-b border-border bg-background">
    <div class="container flex items-center justify-between max-w-7xl">
      <div class="flex space-x-6">
        <div class="flex items-center justify-center">
          <Origami />
          <span class="ml-2 font-semibold max-md:hidden">crow</span>
        </div>

        <NavigationMenu>
          <NavigationMenuList>
            <NavigationMenuItem v-for="route in routes" :key="route.title">
              <router-link :to="route.route">
                <NavigationMenuLink
                  :class="[
                    navigationMenuTriggerStyle(),
                    route.route.name == currentRoute.name ? 'bg-accent' : '',
                  ]"
                  class="font-medium"
                >
                  {{ route.title }}
                </NavigationMenuLink>
              </router-link>
            </NavigationMenuItem>
          </NavigationMenuList>
        </NavigationMenu>
      </div>
      <div>
        <DropdownMenu>
          <DropdownMenuTrigger>
            <Button variant="outline" size="icon">
              <User></User>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuLabel class="flex flex-col space-y-1">
              {{ userName }}
            </DropdownMenuLabel>
            <DropdownMenuSeparator></DropdownMenuSeparator>
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
import { Home, Origami, User } from 'lucide-vue-next'
import {
  NavigationMenu,
  NavigationMenuItem,
  NavigationMenuLink,
  NavigationMenuList,
  navigationMenuTriggerStyle,
} from '@/components/ui/navigation-menu'
import { Button } from '@/components/ui/button'
import { computed } from 'vue'
import router from '@/router'
import { storeToRefs } from 'pinia'
import { useRoute } from 'vue-router'
import { useUserStore } from '@/stores/user.ts'

const currentRoute = useRoute()
const { userInfo } = storeToRefs(useUserStore())
const userName = computed(() => userInfo.value?.displayName)

const routes = computed(() =>
  router.getRoutes().map((route) => ({
    route: route,
    title: (route.meta?.name || route.name) as string,
    icon: route.meta?.icon || Home,
  })),
)
</script>
