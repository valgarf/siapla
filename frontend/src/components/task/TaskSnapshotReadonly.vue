<template>
  <div class="sidebar-content">
    <section class="sidebar-section">
      <div class="text-h5 q-mb-sm">
        {{ snapshot.title }}
        <q-chip color="secondary" text-color="white" class="q-pa-md q-ml-sm">
          {{ snapshot.designation }}
        </q-chip>
      </div>
      <q-markdown v-if="snapshot.description" :src="snapshot.description" />
      <div v-else><i>No description</i></div>
    </section>

    <section
      v-show="snapshot.designation === 'REQUIREMENT'"
      class="sidebar-section responsive-row"
    >
      <div class="row items-baseline">
        <div class="text-subtitle2 q-pr-md">Start:</div>
        <div>{{ formatDatetime(snapshot.earliestStart) }}</div>
      </div>
    </section>

    <section
      v-show="snapshot.designation === 'MILESTONE'"
      class="sidebar-section responsive-row"
    >
      <div class="row items-baseline">
        <div class="text-subtitle2 q-pr-md">Schedule:</div>
        <div>{{ formatDatetime(snapshot.scheduleTarget) }}</div>
      </div>
      <div class="row items-baseline">
        <div class="text-subtitle2 q-pr-md">Priority:</div>
        <div>{{ formatPriority(snapshot.priority) }}</div>
      </div>
    </section>

    <section
      v-show="['TASK', 'GROUP'].includes(snapshot.designation)"
      class="sidebar-section"
    >
      <div class="q-gutter-y-sm">
        <div
          v-for="(option, idx) in snapshot.resourceConstraints"
          :key="idx"
          class="row items-center q-gutter-sm"
        >
          <div class="col">
            <div class="text-subtitle2">
              Resource Constraint {{ idx + 1 }}
            </div>

            <div class="q-gutter-xs q-mt-xs">
              <ResourceChip
                v-for="resource in option.resources"
                :key="resource.dbId"
                :resource="resource"
                :clickable="false"
              />
              <span v-if="option.resources.length === 0" class="text-grey-5">—</span>
            </div>

            <div class="row q-gutter-sm items-center q-mt-xs">
              <div class="q-ml-md text-subtitle2">
                {{ option.optional ? 'Optional' : 'Required' }}
              </div>
              <div class="q-ml-lg text-subtitle2">
                Speed: {{ formatSpeed(option.speed) }}
              </div>
            </div>
          </div>
        </div>

        <div
          v-if="snapshot.resourceConstraints.length === 0"
          class="text-grey-5"
        >
          —
        </div>
      </div>
    </section>

    <section
      v-show="snapshot.designation === 'TASK'"
      class="sidebar-section responsive-row"
    >
      <div class="row items-baseline">
        <div class="text-subtitle2 q-pr-md">Effort:</div>
        <div>{{ snapshot.effort != null ? `${snapshot.effort} days` : '-' }}</div>
      </div>
    </section>

    <section class="sidebar-section column q-gutter-xs">
      <div class="col">
        <div class="text-subtitle2">Predecessors</div>
        <div v-if="snapshot.predecessors.length > 0">
          <TaskChip
            v-for="task in snapshot.predecessors"
            :key="task.dbId"
            :task="task"
            :clickable="false"
          />
        </div>
        <div v-else class="text-grey-5">—</div>
      </div>

      <div class="col">
        <div class="text-subtitle2">Successors</div>
        <div v-if="snapshot.successors.length > 0">
          <TaskChip
            v-for="task in snapshot.successors"
            :key="task.dbId"
            :task="task"
            :clickable="false"
          />
        </div>
        <div v-else class="text-grey-5">—</div>
      </div>

      <div class="col">
        <div class="text-subtitle2">Parent</div>
        <div v-if="snapshot.parent">
          <TaskChip :task="snapshot.parent" :clickable="false" />
        </div>
        <div v-else class="text-grey-5">—</div>
      </div>

      <div class="col">
        <div class="text-subtitle2">Children</div>
        <div v-if="snapshot.children.length > 0">
          <TaskChip
            v-for="task in snapshot.children"
            :key="task.dbId"
            :task="task"
            :clickable="false"
          />
        </div>
        <div v-else class="text-grey-5">—</div>
      </div>

      <div v-show="showRequirementsSection" class="col">
        <div class="text-subtitle2">Requirements</div>
        <div v-if="snapshot.requirements.length > 0">
          <TaskChip
            v-for="task in snapshot.requirements"
            :key="task.dbId"
            :task="task"
            :clickable="false"
          />
        </div>
        <div v-else class="text-grey-5">—</div>
      </div>

      <div v-show="showMilestonesSection" class="col">
        <div class="text-subtitle2">Milestones</div>
        <div v-if="snapshot.milestones.length > 0">
          <TaskChip
            v-for="task in snapshot.milestones"
            :key="task.dbId"
            :task="task"
            :clickable="false"
          />
        </div>
        <div v-else class="text-grey-5">—</div>
      </div>
    </section>

    <section
      v-show="snapshot.designation === 'TASK'"
      class="sidebar-section bookings-section"
    >
      <div class="col">
        <div class="text-subtitle2">Bookings</div>
        <div v-if="snapshot.bookings.length > 0" class="bookings-row">
          <div
            v-for="(booking, idx) in snapshot.bookings"
            :key="booking.dbId ?? idx"
            class="booking-box"
          >
            <q-chip
              v-if="booking.final"
              dense
              color="positive"
              text-color="white"
              class="q-mb-sm"
            >
              Final
            </q-chip>

            <div class="booking-resources q-mb-sm">
              <div class="text-subtitle2 q-mb-xs">Resources</div>
              <div v-if="booking.resources.length > 0" class="q-gutter-xs">
                <ResourceChip
                  v-for="resource in booking.resources"
                  :key="resource.dbId"
                  :resource="resource"
                  :clickable="false"
                />
              </div>
              <div v-else class="text-grey-5">—</div>
            </div>

            <div class="row items-baseline q-mb-xs">
              <div class="text-subtitle2 q-pr-md">Start:</div>
              <div>{{ formatDatetime(booking.start) }}</div>
            </div>
            <div class="row items-baseline">
              <div class="text-subtitle2 q-pr-md">End:</div>
              <div>{{ formatDatetime(booking.end) }}</div>
            </div>
          </div>
        </div>
        <div v-else class="text-grey-5">—</div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import ResourceChip from 'src/components/common/ResourceChip.vue';
import TaskChip from 'src/components/common/TaskChip.vue';
import { formatDatetime } from 'src/common/datetime';
import type { Resource } from 'src/stores/resource';
import type { Task } from 'src/stores/task';

export interface TaskSnapshotReadonlyTaskRef {
  dbId: number;
  title: string;
}

export interface TaskSnapshotReadonlyResourceConstraint {
  resources: Resource[];
  optional: boolean;
  speed: number;
}

export interface TaskSnapshotReadonlyBooking {
  dbId?: number | null;
  start: Date | null;
  end: Date | null;
  final: boolean;
  resources: Resource[];
}

export interface TaskSnapshotReadonlyData {
  dbId: number;
  title: string;
  description: string;
  designation: string;
  earliestStart: Date | null;
  scheduleTarget: Date | null;
  effort: number | null;
  priority: number | null;
  predecessors: Task[];
  successors: Task[];
  parent: Task | null;
  children: Task[];
  resourceConstraints: TaskSnapshotReadonlyResourceConstraint[];
  requirements: Task[];
  milestones: Task[];
  bookings: TaskSnapshotReadonlyBooking[];
}

const props = defineProps<{
  snapshot: TaskSnapshotReadonlyData;
  showRequirements?: boolean;
  showMilestones?: boolean;
}>();

const showRequirementsSection = props.showRequirements ?? true;
const showMilestonesSection = props.showMilestones ?? true;

function formatPriority(priority: number | null): string {
  return priority == null ? '-' : priority.toFixed(2);
}

function formatSpeed(speed: number): string {
  return Number(speed).toFixed(2);
}
</script>

<style scoped>
.sidebar-content {
  display: flex;
  flex-flow: column nowrap;
  gap: 0;
  align-items: stretch;
}

.sidebar-section {
  padding: 12px 12px;
  border-bottom: 1px solid #f0f0f0;
}

.bookings-row {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 8px;
}

.booking-box {
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  padding: 12px;
  background: #fafafa;
}

.booking-resources {
  display: flex;
  flex-direction: column;
}
</style>
