import { defineStore, acceptHMRUpdate } from 'pinia';
import { computed, ref, type Ref } from 'vue';

import { type ResourceConstraint, useTaskStore } from './task';
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

export class TaskHistorySidebarData implements SidebarData {
  readonly kind = 'taskHistory';
  taskId: number;
  constructor(taskId: number) {
    this.taskId = taskId;
  }
  valid(): boolean {
    return true; // task might not be in current revision
  }
}

export interface NewTaskDefaults {
  parentId?: number | null;
  predecessorIds?: number[];
  successorIds?: number[];
  resourceConstraints?: ResourceConstraint[];
}

export class NewTaskSidebarData implements SidebarData {
  parentId: number | null | undefined;
  predecessorIds: number[] | undefined;
  successorIds: number[] | undefined;
  resourceConstraints: ResourceConstraint[] | undefined;
  constructor(defaults?: NewTaskDefaults) {
    if (defaults) {
      this.parentId = defaults.parentId;
      this.predecessorIds = defaults.predecessorIds;
      this.successorIds = defaults.successorIds;
      this.resourceConstraints = defaults.resourceConstraints;
    }
  }
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
  constructor() {}
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
  const isExpanded = ref(false);
  // editing state: reference to the SidebarData being edited (new or existing), null otherwise
  const currentEditing: Ref<SidebarData | null> = ref(null);
  // UI hint to make save/cancel buttons shake when a blocked action occurs
  const shakeButtons = ref(false);

  const activeSidebars = computed(() => stack.value.slice());
  const activeSidebar = computed(() => {
    if (currentEditing.value != null) {
      return currentEditing.value;
    }
    return stack.value[stack.value.length - 1] ?? null;
  });

  // NOTE: selection population is moved out of the store (to the caller components).
  // selection population is done by the caller components (PlanGantt*).

  // forward stack for next()
  const forwardStack: Ref<SidebarData[]> = ref([]);

  function isSameSidebar(a: SidebarData | null, b: SidebarData | null): boolean {
    if (a == null || b == null) return false;
    // task history (must come before generic taskId check)
    if (a instanceof TaskHistorySidebarData && b instanceof TaskHistorySidebarData) {
      return a.taskId === b.taskId;
    }
    if (a instanceof TaskHistorySidebarData || b instanceof TaskHistorySidebarData) {
      return false;
    }
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

  // Toggle visibility of a sidebar entry (open/close). If a different sidebar is provided,
  // open it (push if necessary). If same as active and open -> close.
  function toggle(sidebar: SidebarData) {
    const last = stack.value[stack.value.length - 1] ?? null;
    if (currentEditing.value != null && !isSameSidebar(currentEditing.value, sidebar)) {
      // we are in edit mode, cannot change sidebar
      triggerShake();
      return;
    }
    if (last != null && isSameSidebar(last, sidebar) && isOpen.value) {
      // same sidebar, close it
      isOpen.value = false;
      return;
    }
    // open (and ensure top is this sidebar)
    // if it's already top, just ensure visible
    if (last != null && isSameSidebar(last, sidebar)) {
      isOpen.value = true;
      return;
    }
    pushSidebar(sidebar);
  }
  function pushSidebar(sidebar: SidebarData) {
    // push only existing items (callers must use createNew for new items)
    const last = stack.value[stack.value.length - 1] ?? null;
    if (currentEditing.value != null && !isSameSidebar(last, sidebar)) {
      triggerShake();
      return;
    }
    if (last != null && isSameSidebar(last, sidebar)) {
      isOpen.value = true;
      return;
    }
    // push new top, clear forward history
    stack.value.push(sidebar);
    forwardStack.value = [];
    // limit stack to 20 entries
    while (stack.value.length > 20) {
      stack.value.shift();
    }
    isOpen.value = true;
  }

  // Replace the active sidebar entry (useful after saving to update id). If no entry, push instead.
  function replaceTop(sidebar: SidebarData) {
    const last = stack.value[stack.value.length - 1] ?? null;
    if (currentEditing.value != null && !isSameSidebar(last, sidebar)) {
      triggerShake();
      return;
    }
    if (stack.value.length == 0) {
      pushSidebar(sidebar);
      return;
    }
    stack.value[stack.value.length - 1] = sidebar;
    isOpen.value = true;
  }

  function popSidebar() {
    if (currentEditing.value != null) {
      triggerShake();
      return;
    }
    const popped = stack.value.pop();
    if (popped != null) {
      forwardStack.value.push(popped);
    }
    if (!stack.value.length) {
      // keep the stack but hide sidebar
      isOpen.value = false;
    }
  }

  function back() {
    popSidebar();
  }

  function next() {
    if (currentEditing.value != null) {
      triggerShake();
      return;
    }
    // restore from forwardStack if available
    const f = forwardStack.value.pop();
    if (f) {
      stack.value.push(f);
      isOpen.value = true;
    }
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
    currentEditing.value = null;
    if (sidebar != null) pushSidebar(sidebar);
  }

  function toggleOpen() {
    isOpen.value = !isOpen.value;
  }

  function toggleExpand() {
    isExpanded.value = !isExpanded.value;
  }

  function triggerShake() {
    // ensure sidebar is visible when we signal a blocked action
    isOpen.value = true;
    shakeButtons.value = true;
    // clear after animation timeframe
    setTimeout(() => (shakeButtons.value = false), 600);
  }

  // discard current editing (abort). If creating a new item, drop it. If editing existing, just clear editing.
  function discard() {
    // if current editing is a new item, just clear it
    if (
      currentEditing.value != null &&
      (currentEditing.value.constructor.name === 'NewTaskSidebarData' ||
        currentEditing.value.constructor.name === 'NewResourceSidebarData')
    ) {
      currentEditing.value = null;
      // do not push anything; keep stack untouched
      isOpen.value = false;
      forwardStack.value = [];
      return;
    }
    currentEditing.value = null;
  }

  // start editing an existing or new SidebarData
  function startEdit(sidebar: SidebarData | null) {
    if (currentEditing.value != null) {
      triggerShake();
      return;
    }
    currentEditing.value = sidebar;
    // ensure sidebar visible
    isOpen.value = true;
  }

  // create new item: alias for starting edit of a new SidebarData
  function createNew(sidebar: SidebarData) {
    startEdit(sidebar);
  }

  // save editing: end editing. If a savedSidebar is provided (e.g. new item got an id), push/replace it on the stack.
  function save(savedSidebar: SidebarData) {
    currentEditing.value = null;
    // if top is same, replace, otherwise push
    const last = stack.value[stack.value.length - 1] ?? null;
    if (!isSameSidebar(last, savedSidebar)) {
      pushSidebar(savedSidebar);
    }
  }

  // delete all stack entries matching the provided sidebar (by id). End editing and close sidebar if stack empty.
  function deleteSidebar(sidebar: SidebarData) {
    // remove matching entries
    stack.value = stack.value.filter((s) => !isSameSidebar(s, sidebar));
    // end any editing
    currentEditing.value = null;
    if (!stack.value.length) {
      isOpen.value = false;
    }
  }

  return {
    // state
    stack,
    activeSidebars,
    activeSidebar,
    isOpen,
    isExpanded,
    // actions (keeps previous names for compatibility where reasonable)
    pushSidebar,
    replaceTop,
    toggle,
    back,
    next,
    popSidebar,
    atFirst,
    atLast,
    reset,
    // visibility
    toggleOpen,
    toggleExpand,
    createNew,
    // edit-mode controls
    currentEditing,
    startEdit,
    save,
    discard,
    // deletion
    deleteSidebar,
    // misc
    shakeButtons,
    triggerShake,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useSidebarStore, import.meta.hot));
}
