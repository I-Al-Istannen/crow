<template>
  <div>
    <div ref="parentRef" class="space-y-1">
      <div v-for="entry in draggingModifiers" :key="entry.key" class="flex items-center">
        <TestModifierSelect
          :modifier="entry"
          @update:modifier="Object.assign(entry, $event)"
          class="flex-grow overflow-hidden min-w-1 hover:bg-gray-50"
        />
        <Button
          variant="ghost"
          @click.prevent.stop="removeModifier(entry.key)"
          size="icon"
          class="flex-shrink-0"
        >
          <LucideTrash2 class="text-red-500" />
        </Button>
      </div>
    </div>
    <Button @click.prevent.stop="addNewModifier" variant="ghost" size="sm" class="pl-2">
      <LucidePlus />
      Add modifier
    </Button>
  </div>
</template>

<script setup lang="ts">
import { LucidePlus, LucideTrash2 } from 'lucide-vue-next'
import { ref, toRefs, watch } from 'vue'
import { Button } from '@/components/ui/button'
import type { SortEventData } from '@formkit/drag-and-drop'
import type { TestModifier } from '@/types.ts'
import TestModifierSelect from '@/components/TestModifierSelect.vue'
import { useDragAndDrop } from '@formkit/drag-and-drop/vue'

type KeyedTestModifier = TestModifier & { key: number }

const modifierKeyCounter = ref(0)

const props = defineProps<{
  value: KeyedTestModifier[]
}>()
const { value: passedInModifiers } = toRefs(props)
const emit = defineEmits<{
  'update:value': [modifiers: KeyedTestModifier[]]
}>()

const [parentRef, draggingModifiers] = useDragAndDrop<KeyedTestModifier>([], {
  sortable: true,
  dragHandle: '.drag-handle',
  dropZoneClass: 'saturate-0 opacity-20',
  onSort: (event) => {
    const data = event as unknown as SortEventData<KeyedTestModifier>
    emit('update:value', data.values)
  },
})

watch(passedInModifiers, (newModifiers) => {
  draggingModifiers.value = newModifiers
})

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
