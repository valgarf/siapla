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
