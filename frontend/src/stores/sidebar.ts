import { defineStore, acceptHMRUpdate } from 'pinia';
import { computed, ref, type Ref } from 'vue';

import { useTaskStore } from './task';
import { useResourceStore } from './resource';

export interface SidebarData {
  valid(): boolean;
}

export class TaskSidebarData implements SidebarData {
  taskId: number;
  constructor(task_id: number) {
    this.taskId = task_id;
  }
  valid(): boolean {
    return useTaskStore().task(this.taskId) != null;
  }
}

export class NewTaskSidebarData implements SidebarData {
  constructor() { }
  valid(): boolean {
    return true;
  }
}

export class ResourceSidebarData implements SidebarData {
  resourceId: number;
  constructor(resourceId: number) {
    this.resourceId = resourceId;
  }
  valid(): boolean {
    return useResourceStore().resource(this.resourceId) != null;
  }
}

export class NewResourceSidebarData implements SidebarData {
  constructor() { }
  valid(): boolean {
    return true;
  }
}

// We keep SidebarData classes but simplify the store model to a single sidebar stack
// The stack holds opened items; back/pop moves to the previous item. The sidebar
// can be opened/closed and expanded (full screen).

// actual store
export const useSidebarStore = defineStore('sidebarStore', () => {
  // single stack of opened items in the sidebar
  const stack: Ref<SidebarData[]> = ref([]);

  // sidebar UI state
  const isOpen = ref(false);
  const isSelected = ref(false);
  const isExpanded = ref(false);
  // edit mode state: when true, switching sidebars is blocked
  const isEditing = ref(false);
  // UI hint to make save/cancel buttons shake when a blocked action occurs
  const shakeButtons = ref(false);

  const activeSidebars = computed(() => stack.value.slice());
  const activeSidebar = computed(() => stack.value[stack.value.length - 1]);

  // NOTE: selection population is moved out of the store (to the caller components).
  // selection population is done by the caller components (PlanGantt*).

  // forward stack for next()
  const forwardStack: Ref<SidebarData[]> = ref([]);

  function isSameSidebar(a: SidebarData | null, b: SidebarData | null): boolean {
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

  function toggleSidebar(sidebar: SidebarData) {
    const last = stack.value[stack.value.length - 1] ?? null;
    if (isEditing.value && !isSameSidebar(last, sidebar)) {
      // blocked while editing
      triggerShake();
      return;
    }
    if (last != null && isSameSidebar(last, sidebar) && isOpen.value) {
      isOpen.value = false;
      isSelected.value = false;
    }
    else {
      pushSidebar(sidebar);
    }
  }
  function pushSidebar(sidebar: SidebarData) {
    // if same as last, do nothing
    const last = stack.value[stack.value.length - 1] ?? null;
    if (isEditing.value && !isSameSidebar(last, sidebar)) {
      // blocked while editing
      triggerShake();
      return;
    }
    if (last != null && isSameSidebar(last, sidebar)) {
      isOpen.value = true;
      isSelected.value = true;
      return;
    }
    // push new top, clear forward history
    stack.value.push(sidebar);
    forwardStack.value = [];
    // if this is a new item, enter edit mode
    if (sidebar.constructor.name === 'NewTaskSidebarData' || sidebar.constructor.name === 'NewResourceSidebarData') {
      isEditing.value = true;
    }
    // limit stack to 20 entries
    while (stack.value.length > 20) {
      stack.value.shift();
    }
    isOpen.value = true;
    isSelected.value = true;
  }

  function replaceSidebar(sidebar: SidebarData) {
    const last = stack.value[stack.value.length - 1] ?? null;
    if (isEditing.value && !isSameSidebar(last, sidebar)) {
      triggerShake();
      return;
    }
    if (stack.value.length == 0) {
      pushSidebar(sidebar);
      return;
    }
    stack.value[stack.value.length - 1] = sidebar;
    isOpen.value = true;
    isSelected.value = true;
    // if replaced with a new item, enter edit mode
    if (sidebar.constructor.name === 'NewTaskSidebarData' || sidebar.constructor.name === 'NewResourceSidebarData') {
      isEditing.value = true;
    }
  }

  function popSidebar() {
    if (isEditing.value) {
      // popping while editing is blocked; request button shake
      triggerShake();
      return;
    }
    if (stack.value.length <= 1) {
      // keep the stack but hide sidebar
      isOpen.value = false;
      isSelected.value = false;
      return;
    }
    const popped = stack.value.pop() as SidebarData;
    // push into forward stack so next() can restore
    forwardStack.value.push(popped);
  }

  function back() {
    popSidebar();
  }

  function next() {
    if (isEditing.value) {
      triggerShake();
      return;
    }
    // restore from forwardStack if available
    const f = forwardStack.value.pop();
    if (f) {
      stack.value.push(f);
      isOpen.value = true;
      isSelected.value = true;
    }
  }

  function closeLayer() {
    // hide sidebar but keep stack intact
    isOpen.value = false;
    isSelected.value = false;
  }

  function atFirst(): boolean {
    return stack.value.length <= 1;
  }
  function atLast(): boolean {
    // atLast means there's no forward history
    return forwardStack.value.length == 0;
  }

  function reset(sidebar: SidebarData | null = null) {
    stack.value = [];
    isOpen.value = false;
    isSelected.value = false;
    isEditing.value = false;
    if (sidebar != null) pushSidebar(sidebar);
  }

  function toggleOpen() {
    isOpen.value = !isOpen.value;
    isSelected.value = isOpen.value;
  }

  function toggleExpand() {
    isExpanded.value = !isExpanded.value;
  }

  function triggerShake() {
    // ensure sidebar is visible when we signal a blocked action
    isOpen.value = true;
    isSelected.value = true;
    shakeButtons.value = true;
    // clear after animation timeframe
    setTimeout(() => (shakeButtons.value = false), 600);
  }

  // cancel editing: always allowed. If top is a new item, remove it and close sidebar.
  function cancelEdit() {
    const active = activeSidebar.value ?? null;
    isEditing.value = false;
    // New items should be removed from the stack
    if (active != null && (active.constructor.name === 'NewTaskSidebarData' || active.constructor.name === 'NewResourceSidebarData')) {
      // remove top
      stack.value.pop();
      // close sidebar
      isOpen.value = false;
      isSelected.value = false;
      // clear forward history
      forwardStack.value = [];
    }
  }

  return {
    // state
    stack,
    activeSidebars,
    activeSidebar,
    isOpen,
    isSelected,
    isExpanded,
    // actions (keeps previous names for compatibility)
    pushSidebar,
    toggleSidebar,
    back,
    next,
    closeLayer,
    popSidebar,
    replaceSidebar,
    atFirst,
    atLast,
    reset,
    // new actions
    toggleOpen,
    toggleExpand,
    // edit-mode controls
    isEditing,
    setEditing: (v: boolean) => (isEditing.value = v),
    cancelEdit,
    shakeButtons,
    triggerShake,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useSidebarStore, import.meta.hot));
}
