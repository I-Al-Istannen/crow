<template>
  <div>
    <div ref="parentRef" class="space-y-1">
      <div v-for="entry in draggingModifiers" :key="entry.key" class="flex items-center">
        <TestModifierSelect
          :readonly="readonly"
          :modifier="entry"
          :modifier-target="modifierTarget"
          @update:modifier="!readonly && Object.assign(entry, $event)"
          class="flex-grow overflow-hidden min-w-1 hover:bg-gray-50"
        />
        <Button
          v-if="!readonly"
          @click.prevent.stop="removeModifier(entry.key)"
          variant="ghost"
          size="icon"
          class="flex-shrink-0"
        >
          <LucideTrash2 class="text-red-500" />
        </Button>
      </div>
    </div>
    <Button
      v-if="!readonly"
      @click.prevent.stop="addNewModifier"
      variant="ghost"
      size="sm"
      class="pl-2"
    >
      <LucidePlus />
      Add modifier
    </Button>
    <div v-if="showMultipleWarning" class="pl-2 text-xs text-orange-500 flex items-center">
      <LucideTriangleAlert class="size-4 mr-2" />
      Multiple inputs/outputs will be concatenated in order
    </div>
  </div>
</template>

<script setup lang="ts">
import { LucidePlus, LucideTrash2, LucideTriangleAlert } from 'lucide-vue-next'
import { computed, ref, toRefs, watch } from 'vue'
import { Button } from '@/components/ui/button'
import type { SortEventData } from '@formkit/drag-and-drop'
import type { TestModifier } from '@/types.ts'
import TestModifierSelect from '@/components/TestModifierSelect.vue'
import { useDragAndDrop } from '@formkit/drag-and-drop/vue'

type KeyedTestModifier = TestModifier & { key: number }

const modifierKeyCounter = ref(0)

const props = defineProps<{
  value: KeyedTestModifier[]
  readonly?: boolean
  modifierTarget: 'compiler' | 'binary'
}>()
const { value: passedInModifiers, readonly, modifierTarget } = toRefs(props)
const emit = defineEmits<{
  'update:value': [modifiers: KeyedTestModifier[]]
}>()

const [parentRef, draggingModifiers] = useDragAndDrop<KeyedTestModifier>([], {
  sortable: !readonly.value,
  dragHandle: readonly.value ? '.i-do-not-exist' : '.drag-handle',
  dropZoneClass: 'saturate-0 opacity-20',
  onSort: (event) => {
    const data = event as unknown as SortEventData<KeyedTestModifier>
    emit('update:value', data.values)
  },
})

const showMultipleWarning = computed(
  () =>
    passedInModifiers.value.filter((it) => it.type === 'ProgramInput').length > 1 ||
    passedInModifiers.value.filter((it) => it.type === 'ExpectedOutput').length > 1,
)

watch(
  passedInModifiers,
  (newModifiers) => {
    modifierKeyCounter.value += newModifiers.length
    draggingModifiers.value = newModifiers
  },
  { immediate: true },
)

function addNewModifier() {
  emit('update:value', [
    ...passedInModifiers.value,
    { type: 'ShouldSucceed', key: modifierKeyCounter.value++ },
  ])
}

function removeModifier(key: number) {
  emit(
    'update:value',
    passedInModifiers.value.filter((it) => it.key !== key),
  )
}
</script>
