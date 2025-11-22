import { defineStore, acceptHMRUpdate } from 'pinia';
import { computed, ref, type Ref } from 'vue';

import { useTaskStore } from './task';
import { useResourceStore } from './resource';

export interface DialogData {
  valid(): boolean;
}

export class TaskDialogData implements DialogData {
  taskId: number;
  constructor(task_id: number) {
    this.taskId = task_id;
  }
  valid(): boolean {
    return useTaskStore().task(this.taskId) != null;
  }
}

export class NewTaskDialogData implements DialogData {
  constructor() { }
  valid(): boolean {
    return true;
  }
}

export class ResourceDialogData implements DialogData {
  resourceId: number;
  constructor(resourceId: number) {
    this.resourceId = resourceId;
  }
  valid(): boolean {
    return useResourceStore().resource(this.resourceId) != null;
  }
}

export class NewResourceDialogData implements DialogData {
  constructor() { }
  valid(): boolean {
    return true;
  }
}

// We keep DialogData classes but simplify the store model to a single sidebar stack
// The stack holds opened items; back/pop moves to the previous item. The sidebar
// can be opened/closed and expanded (full screen).

// actual store
export const useDialogStore = defineStore('dialogStore', () => {
  // single stack of opened items in the sidebar
  const stack: Ref<DialogData[]> = ref([]);

  // sidebar UI state
  const isOpen = ref(false);
  const isExpanded = ref(false);

  // selection inputs: filled from the element shown in the sidebar
  const selectedRow: Ref<number | null> = ref(null);
  const selectedElements: Ref<Array<Record<string, unknown>>> = ref([]);

  const activeDialogs = computed(() => stack.value.slice());
  const activeDialog = computed(() => stack.value[stack.value.length - 1]);

  // NOTE: selection population is moved out of the store (to the caller components).
  // selection population is done by the caller components (PlanGantt*).

  // forward stack for next()
  const forwardStack: Ref<DialogData[]> = ref([]);

  function isSameDialog(a: DialogData | null, b: DialogData | null): boolean {
    if (a == null || b == null) return false;
    // task
    if (isObjectWithNumberProp(a, 'taskId') && isObjectWithNumberProp(b, 'taskId')) {
      return a['taskId'] === b['taskId'];
    }
    // resource
    if (isObjectWithNumberProp(a, 'resourceId') && isObjectWithNumberProp(b, 'resourceId')) {
      return a['resourceId'] === b['resourceId'];
    }
    // new items
    if (a.constructor.name === b.constructor.name) return true;
    return false;
  }

  function isObjectWithNumberProp(x: unknown, prop: string): x is Record<string, number> {
    const r = x as Record<string, unknown>;
    return typeof x === 'object' && x !== null && prop in r && typeof r[prop] === 'number';
  }

  function pushDialog(dialog: DialogData) {
    // if same as last, do nothing
    const last = stack.value[stack.value.length - 1] ?? null;
    if (last != null && isSameDialog(last, dialog)) {
      isOpen.value = true;
      return;
    }
    // push new top, clear forward history
    stack.value.push(dialog);
    forwardStack.value = [];
    // limit stack to 20 entries
    while (stack.value.length > 20) {
      stack.value.shift();
    }
    isOpen.value = true;
  }

  function replaceDialog(dialog: DialogData) {
    if (stack.value.length == 0) {
      pushDialog(dialog);
      return;
    }
    stack.value[stack.value.length - 1] = dialog;
    isOpen.value = true;
  }

  function popDialog() {
    if (stack.value.length <= 1) {
      // keep the stack but hide sidebar
      isOpen.value = false;
      return;
    }
    const popped = stack.value.pop() as DialogData;
    // push into forward stack so next() can restore
    forwardStack.value.push(popped);
  }

  function back() {
    popDialog();
  }

  function next() {
    // restore from forwardStack if available
    const f = forwardStack.value.pop();
    if (f) {
      stack.value.push(f);
      isOpen.value = true;
    }
  }

  function openDialog(dialog: DialogData) {
    // alias for pushDialog
    pushDialog(dialog);
  }

  function closeLayer() {
    // hide sidebar but keep stack intact
    isOpen.value = false;
  }

  function atFirst(): boolean {
    return stack.value.length <= 1;
  }
  function atLast(): boolean {
    // atLast means there's no forward history
    return forwardStack.value.length == 0;
  }

  function reset(dialog: DialogData | null = null) {
    stack.value = [];
    isOpen.value = false;
    if (dialog != null) pushDialog(dialog);
  }

  function toggleOpen() {
    isOpen.value = !isOpen.value;
  }

  function toggleExpand() {
    isExpanded.value = !isExpanded.value;
  }

  return {
    // state
    stack,
    activeDialogs,
    activeDialog,
    isOpen,
    isExpanded,
    selectedRow,
    selectedElements,
    // actions (keeps previous names for compatibility)
    pushDialog,
    openDialog,
    back,
    next,
    closeLayer,
    popDialog,
    replaceDialog,
    atFirst,
    atLast,
    reset,
    // new actions
    toggleOpen,
    toggleExpand,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useDialogStore, import.meta.hot));
}
