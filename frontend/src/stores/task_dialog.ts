import { defineStore, acceptHMRUpdate } from 'pinia';
import { computed, ref, type Ref } from 'vue';

import { useTaskStore } from './task';

// actual store
export const useTaskDialogStore = defineStore('taskDialogStore', () => {
  const task_store = useTaskStore();
  const task_dialog_history: Ref<number[]> = ref([]);
  const new_task: Ref<boolean> = ref(false);

  function push_task_dialog(taskId: number) {
    const task = task_store.task(taskId);
    if (task != null) {
      task_dialog_history.value.push(taskId);
    }
  }
  function push_new_task_dialog() {
    new_task.value = true;
  }
  function pop_task_dialog() {
    if (new_task.value) {
      new_task.value = false;
    } else {
      task_dialog_history.value.pop();
    }
  }
  function reset_task_dialog(taskId?: number) {
    new_task.value = false;
    task_dialog_history.value = [];
    if (taskId != null) {
      push_task_dialog(taskId);
    }
  }
  const active_task_dialog = computed(() => {
    if (new_task.value) {
      return {};
    }
    if (task_dialog_history.value.length > 0) {
      const taskId = task_dialog_history.value[task_dialog_history.value.length - 1];
      if (taskId == null) {
        return null;
      }
      return task_store.task(taskId);
    }
    return null;
  });
  const prev_task_dialog = computed(() => {
    if (new_task.value) {
      if (task_dialog_history.value.length > 0) {
        return task_dialog_history.value[task_dialog_history.value.length - 1];
      }
    } else if (task_dialog_history.value.length > 1) {
      const taskId = task_dialog_history.value[task_dialog_history.value.length - 2];
      if (taskId == null) {
        return null;
      }
      return task_store.task(taskId);
    }
    return null;
  });

  return {
    push_task_dialog,
    push_new_task_dialog,
    pop_task_dialog,
    reset_task_dialog,
    active_task_dialog,
    prev_task_dialog,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useTaskDialogStore, import.meta.hot));
}
