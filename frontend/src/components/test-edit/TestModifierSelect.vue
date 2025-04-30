<template>
  <div class="flex items-center gap-1 p-1 rounded-md justify-start">
    <LucideGripVertical class="h-5 flex-shrink-0 drag-handle cursor-grab" />
    <Select
      :model-value="modifierType"
      @update:model-value="update($event as string, stringArg, crashArg, failArg)"
      required
    >
      <SelectTrigger class="max-w-[20ch] py-0 h-7 flex-shrink-1 md:flex-shrink-0 min-w-1">
        <SelectValue placeholder="Select a modifier" />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectItem value="ProgramArgumentFile">Argument File</SelectItem>
          <SelectItem value="ProgramArgument">Argument String</SelectItem>
        </SelectGroup>
        <SelectGroup>
          <SelectItem value="ProgramInput">Program input</SelectItem>
          <SelectItem value="ExpectedOutput">Expected output</SelectItem>
        </SelectGroup>
        <SelectGroup>
          <SelectItem v-if="showCrash" value="ShouldCrash">Should crash</SelectItem>
          <SelectItem v-if="showFail" value="ShouldFail">Should fail</SelectItem>
          <SelectItem value="ShouldSucceed">Should succeed</SelectItem>
        </SelectGroup>
      </SelectContent>
    </Select>
    <Input
      type="text"
      :placeholder="argPlaceholderText"
      :model-value="stringArg"
      @update:model-value="update(modifierType, $event as string, crashArg, failArg)"
      v-if="hasShortStringArg"
      class="py-0 h-7 min-w-1 text-ellipsis"
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
      <PopoverContent class="w-[90dvw] sm:w-[70dvw] max-w-[120ch]">
        <Textarea
          :model-value="stringArg"
          @update:model-value="update(modifierType, $event as string, crashArg, failArg)"
          class="font-mono whitespace-pre overflow-scroll max-h-[100dvh]"
          rows="10"
          :placeholder="argPlaceholderText"
        />
        <PopoverArrow class="fill-white stroke-gray-200" />
      </PopoverContent>
    </Popover>
    <Select
      v-if="hasCrashArg"
      :model-value="crashArg"
      @update:model-value="update(modifierType, stringArg, $event as CrashSignal, failArg)"
      required
    >
      <SelectTrigger class="py-0 h-7 min-w-1">
        <SelectValue placeholder="Select a crash argument" />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectItem value="FloatingPointException">Floating point exception</SelectItem>
          <SelectItem value="SegmentationFault">Segmentation fault</SelectItem>
        </SelectGroup>
      </SelectContent>
    </Select>
    <Select
      v-if="hasFailArg"
      :model-value="failArg"
      @update:model-value="update(modifierType, stringArg, crashArg, $event as CompilerFailReason)"
      required
    >
      <SelectTrigger class="py-0 h-7 min-w-1">
        <SelectValue placeholder="Select a failure reason" />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectItem value="Parsing">Parsing</SelectItem>
          <SelectItem value="SemanticAnalysis">Semantic analysis</SelectItem>
        </SelectGroup>
      </SelectContent>
    </Select>
  </div>
</template>

<script setup lang="ts">
import type { CompilerFailReason, CrashSignal, TestModifier } from '@/types.ts'
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
import { Textarea } from '@/components/ui/textarea'

const modifier = defineModel<TestModifier>('modifier', { required: true })

const props = defineProps<{
  modifierTarget: 'compiler' | 'binary'
}>()
const { modifierTarget } = toRefs(props)

const showFail = computed(() => {
  return modifierTarget.value === 'compiler' || modifier.value.type === 'ShouldFail'
})
const showCrash = computed(() => {
  return modifierTarget.value === 'binary' || modifier.value.type === 'ShouldCrash'
})

const modifierType = computed(() => modifier.value?.type ?? 'ShouldSucceed')
const stringArg = computed(() => {
  switch (modifier.value.type) {
    case 'ProgramArgument':
      return modifier.value.arg
    case 'ProgramArgumentFile':
      return modifier.value.contents
    case 'ExpectedOutput':
      return modifier.value.output
    case 'ProgramInput':
      return modifier.value.input
    default:
      return ''
  }
})
const crashArg = computed(() => {
  switch (modifier.value.type) {
    case 'ShouldCrash':
      return modifier.value.signal
    default:
      return 'FloatingPointException'
  }
})
const failArg = computed(() => {
  switch (modifier.value.type) {
    case 'ShouldFail':
      return modifier.value.reason
    default:
      return 'Parsing'
  }
})
const hasShortStringArg = computed(() => modifier.value.type === 'ProgramArgument')
const hasLongStringArg = computed(
  () =>
    modifier.value.type === 'ExpectedOutput' ||
    modifier.value.type === 'ProgramInput' ||
    modifier.value.type === 'ProgramArgumentFile',
)
const hasCrashArg = computed(() => modifier.value.type === 'ShouldCrash')
const hasFailArg = computed(() => modifier.value.type === 'ShouldFail')

function update(
  type: string,
  stringVal: string,
  crashVal: CrashSignal,
  failVal: CompilerFailReason,
) {
  switch (type) {
    case 'ProgramArgument':
      modifier.value = { type, arg: stringVal }
      break
    case 'ProgramArgumentFile':
      modifier.value = { type, contents: stringVal }
      break
    case 'ExpectedOutput':
      modifier.value = { type, output: stringVal }
      break
    case 'ProgramInput':
      modifier.value = { type, input: stringVal }
      break
    case 'ShouldCrash':
      modifier.value = { type, signal: crashVal }
      break
    case 'ShouldFail':
      modifier.value = { type, reason: failVal }
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
    case 'ProgramArgumentFile':
      return 'File contents...'
    case 'ProgramInput':
      return 'Input...'
    case 'ExpectedOutput':
      return 'Output...'
    default:
      return ''
  }
})
</script>
