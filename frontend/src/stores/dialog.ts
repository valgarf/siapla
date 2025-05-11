import { defineStore, acceptHMRUpdate } from 'pinia';
import { computed, ref, type Ref } from 'vue';

import { useTaskStore } from './task';

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
  constructor() {}
  valid(): boolean {
    return true;
  }
}

export class ResourceDialogData implements DialogData {
  resource_id: number;
  constructor(resource_id: number) {
    this.resource_id = resource_id;
  }
  valid(): boolean {
    return true;
    // return useTaskStore().task(this.task_id) != null;
  }
}

export class DialogLayer {
  dialogs: DialogData[];
  idx: number;
  active: number;
  constructor(idx: number, dialog: DialogData) {
    this.dialogs = [dialog];
    this.idx = idx;
    this.active = 0;
  }

  activeDialog(): DialogData {
    return this.dialogs[this.active] as DialogData;
  }

  pushDialog(dialog: DialogData) {
    this.dialogs = this.dialogs.slice(0, this.active + 1);
    this.dialogs.push(dialog);
    this.active += 1;
  }

  back() {
    if (this.active > 0) {
      this.active--;
    }
  }

  popDialog() {
    if (this.active > 0) {
      this.dialogs = this.dialogs.slice(0, this.active);
      this.active--;
    }
  }

  replaceDialog(dialog: DialogData) {
    this.dialogs[this.active] = dialog;
  }

  next() {
    if (this.active < this.dialogs.length - 1) {
      this.active++;
    }
  }

  atFirst() {
    return this.active == 0;
  }

  atLast() {
    return this.active == this.dialogs.length - 1;
  }
}

// actual store
export const useDialogStore = defineStore('dialogStore', () => {
  // all dialogs (layer,idx in layer -> DialogData)
  const layers: Ref<DialogLayer[]> = ref([]);

  const activeDialogs = computed(() => {
    return layers.value.map((l) => l.activeDialog());
  });

  const activeDialog = computed(() => {
    return layers.value[layers.value.length - 1]?.activeDialog();
  });

  function pushDialog(dialog: DialogData, layer_idx: number | null = null) {
    if (layers.value.length == 0) {
      openDialog(dialog);
      return;
    }
    if (layer_idx == null || layer_idx < 0 || layer_idx > layers.value.length - 1) {
      layer_idx = layers.value.length - 1;
    }
    const layer = layers.value[layer_idx] as DialogLayer;
    layer.pushDialog(dialog);
  }
  function openDialog(dialog: DialogData) {
    layers.value.push(new DialogLayer(layers.value.length, dialog));
  }
  const activeLayer = computed(() => {
    return layers.value[layers.value.length - 1];
  });

  function popDialog() {
    if (activeLayer.value?.dialogs.length == 1) {
      closeLayer();
    } else {
      activeLayer.value?.popDialog();
    }
  }

  function replaceDialog(dialog: DialogData) {
    activeLayer.value?.replaceDialog(dialog);
  }

  function back(layerIdx: number | null = null) {
    layerIdx = layerIdx ?? layers.value.length - 1;
    layers.value[layerIdx]?.back();
  }

  function next(layerIdx: number | null = null) {
    layerIdx = layerIdx ?? layers.value.length - 1;
    layers.value[layerIdx]?.next();
  }

  function closeLayer(layerIdx: number | null = null) {
    layerIdx = layerIdx ?? layers.value.length - 1;
    const num_close = layers.value.length - layerIdx;
    for (let i = 0; i < num_close; i++) {
      layers.value.pop();
    }
  }

  function atFirst(layerIdx: number | null = null): boolean {
    layerIdx = layerIdx ?? layers.value.length - 1;
    return layers.value[layerIdx]?.atFirst() ?? false;
  }
  function atLast(layerIdx: number | null = null): boolean {
    layerIdx = layerIdx ?? layers.value.length - 1;
    return layers.value[layerIdx]?.atLast() ?? false;
  }

  function reset(dialog: DialogData | null = null) {
    layers.value = [];
    if (dialog != null) {
      pushDialog(dialog);
    }
  }

  return {
    layers: layers,
    activeDialogs,
    activeLayer,
    activeDialog,
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
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useDialogStore, import.meta.hot));
}
