<template>
  <div
    class="flex max-sm:flex-wrap items-center gap-1 p-1 rounded-md justify-start"
    :class="[readonly ? 'ml-3' : '']"
  >
    <LucideGripVertical v-if="!readonly" class="h-5 flex-shrink-0 drag-handle cursor-grab" />
    <SlotOrReadonly
      :readonly="readonly || false"
      :label="currentModifierData.label"
      :class="[readonly ? 'w-[18ch]' : '']"
    >
      <Select
        :model-value="modifierType"
        @update:model-value="modifier.type = $event as TestModifier['type']"
        :class="[readonly ? 'ml-2' : '']"
      >
        <SelectTrigger class="max-w-[20ch] py-0 h-7 flex-shrink-1 md:flex-shrink-0 min-w-1">
          <SelectValue placeholder="Select a modifier" />
        </SelectTrigger>
        <SelectContent>
          <template v-for="group in modifierGroups" :key="group">
            <SelectGroup v-if="group.filter(showModifier).length > 0">
              <SelectItem
                v-for="modifier in group.filter(showModifier)"
                :key="modifier"
                :value="modifier"
              >
                {{ ALL_MODIFIERS[modifier].label }}
              </SelectItem>
            </SelectGroup>
          </template>
        </SelectContent>
      </Select>
    </SlotOrReadonly>
    <Input
      type="text"
      :placeholder="currentModifierData.placeholder"
      :model-value="currentValue"
      @update:model-value="currentModifierData.update(modifier as any, $event as string)"
      v-if="currentModifierData.argType === 'short-string'"
      class="py-0 h-7 min-w-1 text-ellipsis"
    />
    <SlotOrReadonly
      :readonly="readonly || false"
      :label="currentValue + ''"
      v-if="currentModifierData.argType === 'number'"
    >
      <Input
        type="number"
        min="0"
        max="255"
        :model-value="currentValue"
        @update:model-value="currentModifierData.update(modifier as any, $event as number)"
        class="py-0 h-7 min-w-1"
        :class="[
          (currentValue as number) > 255 || (currentValue as number) < 0
            ? 'ring-1 !ring-destructive'
            : '',
        ]"
      />
    </SlotOrReadonly>
    <Popover v-if="currentModifierData.argType === 'long-string'">
      <PopoverTrigger class="h-7 w-full" as-child>
        <Button class="bg-transparent min-w-1 justify-start" variant="outline">
          <span
            v-if="(currentValue as string).length > 0"
            class="overflow-ellipsis overflow-hidden"
          >
            {{ currentValue }}
          </span>
          <span v-else class="text-muted-foreground">
            {{ currentModifierData.placeholder }}
          </span>
        </Button>
      </PopoverTrigger>
      <PopoverContent class="w-[90dvw] sm:w-[70dvw] max-w-[120ch]">
        <Textarea
          :model-value="currentValue"
          @update:model-value="currentModifierData.update(modifier as any, $event as string)"
          class="font-mono whitespace-pre overflow-scroll max-h-[100dvh]"
          rows="10"
          :placeholder="currentModifierData.placeholder"
          :readonly="readonly || false"
        />
        <PopoverArrow class="fill-white stroke-gray-200" />
      </PopoverContent>
    </Popover>
    <SlotOrReadonly
      :readonly="readonly || false"
      :label="currentModifierData.valueLabel!(modifier as any)"
      v-if="currentModifierData.argType === 'select-crash'"
    >
      <Select
        :model-value="currentValue"
        @update:model-value="currentModifierData.update(modifier as any, $event as CrashSignal)"
        required
      >
        <SelectTrigger class="py-0 h-7 min-w-1">
          <SelectValue placeholder="Select a crash argument" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem
              v-for="arg in ['FloatingPointException', 'SegmentationFault']"
              :value="arg"
              :key="arg"
            >
              {{ ALL_MODIFIERS['ShouldCrash'].valueLabel!(arg as any) }}
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    </SlotOrReadonly>
    <SlotOrReadonly
      :readonly="readonly || false"
      :label="currentModifierData.valueLabel!(modifier as any)"
      v-if="currentModifierData.argType === 'select-fail'"
    >
      <Select
        :model-value="currentValue"
        @update:model-value="
          currentModifierData.update(modifier as any, $event as CompilerFailReason)
        "
        required
      >
        <SelectTrigger class="py-0 h-7 min-w-1">
          <SelectValue placeholder="Select a failure reason" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem v-for="arg in ['Parsing', 'SemanticAnalysis']" :value="arg" :key="arg">
              {{ ALL_MODIFIERS['ShouldFail'].valueLabel!(arg as any) }}
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    </SlotOrReadonly>
  </div>
</template>

<script setup lang="ts">
import type { CompilerFailReason, CrashSignal, ModifierValue, TestModifier } from '@/types.ts'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { computed, toRefs, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { LucideGripVertical } from 'lucide-vue-next'
import { PopoverArrow } from 'reka-ui'
import SlotOrReadonly from '@/components/test-edit/SlotOrReadonly.vue'
import { Textarea } from '@/components/ui/textarea'

const modifier = defineModel<TestModifier>('modifier', { required: true })

const props = defineProps<{
  modifierTarget: 'compiler' | 'binary'
  readonly?: boolean
}>()
const { modifierTarget } = toRefs(props)

const modifierType = computed<TestModifier['type']>(() => modifier.value?.type ?? 'ShouldSucceed')

function showModifier(modifierType: TestModifier['type']): boolean {
  if (ALL_MODIFIERS[modifierType].applicableTo.includes(modifierTarget.value)) {
    return true
  }
  return modifierType == modifier.value.type
}

const currentValue = computed(() => {
  const valueFunc = ALL_MODIFIERS[modifier.value.type].value as (
    mod: TestModifier,
  ) => string | number
  return valueFunc(modifier.value)
})

const currentModifierData = computed(() => {
  return ALL_MODIFIERS[modifierType.value] as ModifierHandler<{
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    type: any
  }>
})

const modifierGroups: TestModifier['type'][][] = [
  ['ProgramArgumentFile', 'ProgramArgument'],
  ['ProgramInput', 'ExpectedOutput'],
  ['ExitCode', 'ShouldCrash', 'ShouldTimeout', 'ShouldFail', 'ShouldSucceed'],
]

interface ModifierHandler<T extends TestModifier> {
  update: (modifier: TestModifier & T, val: ModifierValue<T>) => void
  value: (modifier: TestModifier & T) => ModifierValue<T>
  applicableTo: (typeof props)['modifierTarget'][]
  label: string
  argType: 'short-string' | 'long-string' | 'number' | 'select-fail' | 'select-crash' | 'none'
  init: (modifier: Partial<TestModifier> & T) => void
  valueLabel?: (modifier: TestModifier & T) => string
  placeholder?: string
}

const ALL_MODIFIERS: {
  [modifier in TestModifier['type']]: ModifierHandler<Extract<TestModifier, { type: modifier }>>
} = {
  ShouldSucceed: {
    update: () => {},
    value: () => undefined,
    init: () => {},
    applicableTo: ['compiler', 'binary'],
    label: 'Should succeed',
    argType: 'none',
  },
  ShouldFail: {
    update: (modifier, val) => {
      console.log('UPDATING MODIFIER')
      modifier.reason = val
    },
    value: (modifier) => modifier.reason,
    init: (modifier) => (modifier.reason = 'Parsing'),
    applicableTo: ['compiler'],
    label: 'Should fail',
    argType: 'select-fail',
    valueLabel: (mod) => (mod.reason === 'Parsing' ? 'Parsing' : 'Semantic analysis'),
  },
  ShouldCrash: {
    update: (modifier, val) => (modifier.signal = val),
    value: (modifier) => modifier.signal,
    init: (modifier) => (modifier.signal = 'FloatingPointException'),
    applicableTo: ['binary'],
    label: 'Should crash',
    argType: 'select-crash',
    valueLabel: (mod) =>
      mod.signal === 'FloatingPointException' ? 'Floating point exception' : 'Segmentation fault',
  },
  ShouldTimeout: {
    update: () => {},
    value: () => undefined,
    init: () => {},
    applicableTo: ['binary'],
    label: 'Should not terminate',
    argType: 'none',
  },
  ExitCode: {
    update: (modifier, val) => (modifier.code = val),
    value: (modifier) => modifier.code,
    init: (modifier) => (modifier.code = 0),
    applicableTo: ['binary'],
    placeholder: 'Exit code...',
    label: 'Exit code',
    argType: 'number',
  },

  ProgramInput: {
    update: (modifier, val) => (modifier.input = val),
    value: (modifier) => modifier.input,
    init: (modifier) => (modifier.input = ''),
    applicableTo: ['binary'],
    placeholder: 'Input...',
    label: 'Program input',
    argType: 'long-string',
  },
  ExpectedOutput: {
    update: (modifier, val) => (modifier.output = val),
    value: (modifier) => modifier.output,
    init: (modifier) => (modifier.output = ''),
    applicableTo: ['binary'],
    placeholder: 'Output...',
    label: 'Expected output',
    argType: 'long-string',
  },

  ProgramArgument: {
    update: (modifier, val) => (modifier.arg = val),
    value: (modifier) => modifier.arg,
    init: (modifier) => (modifier.arg = ''),
    applicableTo: [],
    placeholder: 'Argument...',
    label: 'Argument string',
    argType: 'short-string',
  },
  ProgramArgumentFile: {
    update: (modifier, val) => (modifier.contents = val),
    value: (modifier) => modifier.contents,
    init: (modifier) => (modifier.contents = ''),
    applicableTo: ['compiler'],
    placeholder: 'File contents...',
    label: 'Argument file',
    argType: 'long-string',
  },
}

// Initialize the modifiers value in the modifier ref
watch(
  modifierType,
  () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    if (currentModifierData.value.value(modifier.value as any) === undefined) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      currentModifierData.value.init(modifier.value as any)
    }
  },
  { immediate: true },
)
</script>
