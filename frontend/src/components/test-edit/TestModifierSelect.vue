<template>
  <div class="flex items-center gap-1 p-1 rounded-md justify-start">
    <LucideGripVertical v-if="!readonly" class="h-5 flex-shrink-0 drag-handle cursor-grab" />
    <SlotOrReadonly :readonly="readonly || false" :label="modifierLabel(modifierType)">
      <Select
        :model-value="modifierType"
        @update:model-value="update($event as string, stringArg, intArg, crashArg, failArg)"
        :class="[readonly ? 'ml-2' : '']"
      >
        <SelectTrigger class="max-w-[20ch] py-0 h-7 flex-shrink-1 md:flex-shrink-0 min-w-1">
          <SelectValue placeholder="Select a modifier" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem value="ProgramArgumentFile">
              {{ modifierLabel('ProgramArgumentFile') }}
            </SelectItem>
            <SelectItem value="ProgramArgument">{{ modifierLabel('ProgramArgument') }}</SelectItem>
          </SelectGroup>
          <SelectGroup>
            <SelectItem value="ProgramInput">{{ modifierLabel('ProgramInput') }}</SelectItem>
            <SelectItem value="ExpectedOutput">{{ modifierLabel('ExpectedOutput') }}</SelectItem>
          </SelectGroup>
          <SelectGroup>
            <SelectItem v-if="showCrash" value="ExitCode">
              {{ modifierLabel('ExitCode') }}
            </SelectItem>
            <SelectItem v-if="showCrash" value="ShouldCrash">
              {{ modifierLabel('ShouldCrash') }}
            </SelectItem>
            <SelectItem v-if="showFail" value="ShouldFail">
              {{ modifierLabel('ShouldFail') }}
            </SelectItem>
            <SelectItem value="ShouldSucceed">{{ modifierLabel('ShouldSucceed') }}</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    </SlotOrReadonly>
    <Input
      type="text"
      :placeholder="argPlaceholderText"
      :model-value="stringArg"
      @update:model-value="update(modifierType, $event as string, intArg, crashArg, failArg)"
      v-if="hasShortStringArg"
      class="py-0 h-7 min-w-1 text-ellipsis"
    />
    <SlotOrReadonly :readonly="readonly || false" :label="intArg + ''" v-if="hasIntArg">
      <Input
        type="number"
        min="0"
        max="255"
        :model-value="intArg"
        @update:model-value="update(modifierType, stringArg, $event as number, crashArg, failArg)"
        class="py-0 h-7 min-w-1"
        :class="[intArg > 255 || intArg < 0 ? 'ring-1 !ring-destructive' : '']"
      />
    </SlotOrReadonly>
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
          @update:model-value="update(modifierType, $event as string, intArg, crashArg, failArg)"
          class="font-mono whitespace-pre overflow-scroll max-h-[100dvh]"
          rows="10"
          :placeholder="argPlaceholderText"
          :readonly="readonly || false"
        />
        <PopoverArrow class="fill-white stroke-gray-200" />
      </PopoverContent>
    </Popover>
    <SlotOrReadonly
      :readonly="readonly || false"
      :label="crashSignalLabel(crashArg)"
      v-if="hasCrashArg"
    >
      <Select
        :model-value="crashArg"
        @update:model-value="
          update(modifierType, stringArg, intArg, $event as CrashSignal, failArg)
        "
        required
      >
        <SelectTrigger class="py-0 h-7 min-w-1">
          <SelectValue placeholder="Select a crash argument" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem value="FloatingPointException">
              {{ crashSignalLabel('FloatingPointException') }}
            </SelectItem>
            <SelectItem value="SegmentationFault">
              {{ crashSignalLabel('SegmentationFault') }}
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    </SlotOrReadonly>
    <SlotOrReadonly
      :readonly="readonly || false"
      :label="compilerFailReasonLabel(failArg)"
      v-if="hasFailArg"
    >
      <Select
        :model-value="failArg"
        @update:model-value="
          update(modifierType, stringArg, intArg, crashArg, $event as CompilerFailReason)
        "
        required
      >
        <SelectTrigger class="py-0 h-7 min-w-1">
          <SelectValue placeholder="Select a failure reason" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem value="Parsing">{{ compilerFailReasonLabel('Parsing') }}</SelectItem>
            <SelectItem value="SemanticAnalysis">
              {{ compilerFailReasonLabel('SemanticAnalysis') }}
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    </SlotOrReadonly>
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
import SlotOrReadonly from '@/components/SlotOrReadonly.vue'
import { Textarea } from '@/components/ui/textarea'

const modifier = defineModel<TestModifier>('modifier', { required: true })

const props = defineProps<{
  modifierTarget: 'compiler' | 'binary'
  readonly?: boolean
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
const intArg = computed(() => {
  switch (modifier.value.type) {
    case 'ExitCode':
      return modifier.value.code
    default:
      return 0
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
const hasIntArg = computed(() => modifier.value.type === 'ExitCode')
const hasCrashArg = computed(() => modifier.value.type === 'ShouldCrash')
const hasFailArg = computed(() => modifier.value.type === 'ShouldFail')

function update(
  type: string,
  stringVal: string,
  intVal: number,
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
    case 'ExitCode':
      modifier.value = { type, code: intVal }
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
    case 'ExitCode':
      return 'Exit code...'
    default:
      return ''
  }
})

function modifierLabel(value: TestModifier['type']) {
  if (value === 'ProgramArgumentFile') {
    return 'Argument file'
  } else if (value === 'ProgramArgument') {
    return 'Argument string'
  } else if (value === 'ProgramInput') {
    return 'Program input'
  } else if (value === 'ExpectedOutput') {
    return 'Expected output'
  } else if (value === 'ShouldCrash') {
    return 'Should crash'
  } else if (value === 'ExitCode') {
    return 'Exit code'
  } else if (value === 'ShouldFail') {
    return 'Should fail'
  } else if (value === 'ShouldSucceed') {
    return 'Should succeed'
  }
  return 'Unknown: ' + value
}

function crashSignalLabel(value: CrashSignal): string {
  if (value === 'FloatingPointException') {
    return 'Floating point exception'
  } else if (value == 'SegmentationFault') {
    return 'Segmentation fault'
  }
  return 'Unknown: ' + value
}

function compilerFailReasonLabel(value: CompilerFailReason): string {
  if (value === 'Parsing') {
    return 'Parsing'
  } else if (value == 'SemanticAnalysis') {
    return 'Semantic analysis'
  }
  return 'Unknown: ' + value
}
</script>
