import type { Task } from "../shared/board";

export const groupOrder = ["active", "blocked", "todo", "done", "unknown"] as const;
export type TaskGroup = (typeof groupOrder)[number];

export function taskGroup(status: string): TaskGroup {
  if (status === "IN PROGRESS") return "active";
  if (status === "BLOCKED") return "blocked";
  if (status === "DONE") return "done";
  if (status === "TODO") return "todo";
  return "unknown";
}

export function dashboardSummary(tasks: readonly Pick<Task, "status">[]) {
  const counts: Record<TaskGroup, number> = {
    active: 0,
    blocked: 0,
    todo: 0,
    done: 0,
    unknown: 0,
  };
  for (const task of tasks) counts[taskGroup(task.status)] += 1;
  const percentage = tasks.length === 0 ? 0 : Math.round((counts.done / tasks.length) * 100);
  return { counts, percentage, total: tasks.length };
}

export function groupTasks(tasks: readonly Task[]): Record<TaskGroup, Task[]> {
  const groups: Record<TaskGroup, Task[]> = {
    active: [],
    blocked: [],
    todo: [],
    done: [],
    unknown: [],
  };
  for (const task of tasks) groups[taskGroup(task.status)].push(task);
  return groups;
}

export function taskTitle(task: Pick<Task, "id" | "title">): string {
  return task.title.replace(new RegExp(`^${task.id}\\b[\\s:–—-]*`), "").trim() || task.title;
}
