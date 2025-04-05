<template>
  <div class="flex items-center gap-1 p-1 rounded-md justify-start">
    <LucideGripVertical class="h-5 flex-shrink-0 drag-handle cursor-grab" />
    <Select
      :model-value="modifierType"
      @update:model-value="update($event as string, stringArg, intArg)"
      required
    >
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
      :model-value="stringArg"
      @update:model-value="update(modifierType, $event as string, intArg)"
      v-if="hasShortStringArg"
      class="py-0 h-7 min-w-1 text-ellipsis"
    />
    <Input
      type="number"
      :placeholder="argPlaceholderText"
      :model-value="intArg"
      @update:model-value="update(modifierType, stringArg, $event as number)"
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
          :model-value="stringArg"
          @update:model-value="update(modifierType, $event as string, intArg)"
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
import { computed, toRefs } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { LucideGripVertical } from 'lucide-vue-next'
import { PopoverArrow } from 'reka-ui'
import type { TestModifier } from '@/types.ts'
import { Textarea } from '@/components/ui/textarea'

const props = defineProps<{
  readonly?: boolean
}>()
const { readonly } = toRefs(props)
const modifier = defineModel<TestModifier>('modifier', { required: true })

const modifierType = computed(() => modifier.value?.type ?? 'ShouldSucceed')
const stringArg = computed(() => {
  switch (modifier.value.type) {
    case 'ProgramArgument':
      return modifier.value.arg
    case 'ExpectedOutput':
      return modifier.value.output
    case 'ProgramInput':
      return modifier.value.input
    default:
      return ''
  }
})
const intArg = computed(() => {
  switch (modifier.value.type) {
    case 'ExitCode':
      return modifier.value.code
    default:
      return 0
  }
})
const hasShortStringArg = computed(() => modifier.value.type === 'ProgramArgument')
const hasLongStringArg = computed(
  () => modifier.value.type === 'ExpectedOutput' || modifier.value.type === 'ProgramInput',
)
const hasIntArg = computed(() => modifier.value.type === 'ExitCode')

function update(type: string, stringVal: string, intVal: number) {
  switch (type) {
    case 'ProgramArgument':
      modifier.value = { type, arg: stringVal }
      break
    case 'ExpectedOutput':
      modifier.value = { type, output: stringVal }
      break
    case 'ProgramInput':
      modifier.value = { type, input: stringVal }
      break
    case 'ExitCode':
      modifier.value = { type, code: intVal }
      break
    case 'ShouldCrash':
      modifier.value = { type }
      break
    case 'ShouldSucceed':
      modifier.value = { type }
      break
  }
}

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
