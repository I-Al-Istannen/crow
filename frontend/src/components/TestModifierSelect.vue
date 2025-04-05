<template>
  <div class="flex items-center gap-1 p-1 rounded-md justify-start">
    <LucideGripVertical class="h-5 flex-shrink-0 drag-handle cursor-grab" />
    <Select v-model="modifierType">
      <SelectTrigger class="max-w-[20ch] py-0 h-7 flex-shrink-1 md:flex-shrink-0 min-w-1">
        <SelectValue placeholder="Select a modifier" />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectItem value="ExitCode">Exit code</SelectItem>
          <SelectItem value="ExpectedOutput">Expected output</SelectItem>
          <SelectItem value="ProgramArgument">Program argument</SelectItem>
          <SelectItem value="ProgramInput">Program input</SelectItem>
          <SelectItem value="ShouldCrash">Should crash</SelectItem>
          <SelectItem value="ShouldSucceed">Should succeed</SelectItem>
        </SelectGroup>
      </SelectContent>
    </Select>
    <Input
      type="text"
      :placeholder="argPlaceholderText"
      v-model="stringArg"
      v-if="hasShortStringArg"
      class="py-0 h-7 min-w-1 text-ellipsis"
    />
    <Input
      type="number"
      :placeholder="argPlaceholderText"
      v-model="intArg"
      v-if="hasIntArg"
      class="py-0 h-7 min-w-1"
    />
    <Popover v-if="hasLongStringArg">
      <PopoverTrigger class="h-7 w-full" as-child>
        <Button class="bg-transparent min-w-1 justify-start" variant="outline">
          <span v-if="stringArg.length > 0" class="overflow-ellipsis overflow-hidden">
            {{ stringArg }}
          </span>
          <span v-else class="text-muted-foreground">{{ argPlaceholderText }}</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent class="w-[70dvw] max-w-[120ch]">
        <Textarea
          v-model="stringArg"
          class="font-mono whitespace-pre overflow-scroll max-h-[100dvh]"
          rows="10"
          :placeholder="argPlaceholderText"
        />
        <PopoverArrow class="fill-white stroke-gray-200" />
      </PopoverContent>
    </Popover>
  </div>
</template>

<script setup lang="ts">
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { LucideGripVertical } from 'lucide-vue-next'
import { PopoverArrow } from 'reka-ui'
import type { TestModifier } from '@/types.ts'
import { Textarea } from '@/components/ui/textarea'

const modifierType = ref<string>('ExpectedOutput')
const stringArg = ref<string>('')
const intArg = ref<number>(0)

const modifier = defineModel<TestModifier>('modifier')

watch(
  modifier,
  (newMod) => {
    if (newMod) {
      modifierType.value = newMod.type
      switch (newMod.type) {
        case 'ProgramArgument':
          stringArg.value = newMod.arg
          break
        case 'ExpectedOutput':
          stringArg.value = newMod.output
          break
        case 'ProgramInput':
          stringArg.value = newMod.input
          break
        case 'ExitCode':
          intArg.value = newMod.code
          break
      }
    }
  },
  { immediate: true },
)

watch([modifierType, stringArg, intArg], ([type, string, int]) => {
  switch (type) {
    case 'ProgramArgument':
      modifier.value = { type, arg: string }
      break
    case 'ExpectedOutput':
      modifier.value = { type, output: string }
      break
    case 'ProgramInput':
      modifier.value = { type, input: string }
      break
    case 'ExitCode':
      modifier.value = { type, code: int }
      break
    case 'ShouldCrash':
      modifier.value = { type }
      break
    case 'ShouldSucceed':
      modifier.value = { type }
      break
  }
})

const hasShortStringArg = computed(() => modifierType.value === 'ProgramArgument')
const hasLongStringArg = computed(
  () => modifierType.value === 'ExpectedOutput' || modifierType.value === 'ProgramInput',
)
const hasIntArg = computed(() => modifierType.value === 'ExitCode')

const argPlaceholderText = computed(() => {
  switch (modifierType.value) {
    case 'ProgramArgument':
      return 'Argument...'
    case 'ExpectedOutput':
      return 'Output...'
    case 'ProgramInput':
      return 'Input...'
    case 'ExitCode':
      return 'Exit code...'
    default:
      return ''
  }
})
</script>
