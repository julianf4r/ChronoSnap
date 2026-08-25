export interface Tag {
  id: number;
  name: string;
  parent_id: number | null;
  color: string;
}

export interface DBEvent {
  id: number;
  date: string;
  start_minute: number;
  end_minute: number;
  main_tag_id: number;
  sub_tag_id: number | null;
  content: string;
}

export interface Reminder {
  id: number;
  date: string;
  minute: number;
  content: string;
  is_completed: boolean;
}

export interface PlanTask {
  id: number;
  title: string;
  start_date: string;
  end_date: string;
  main_tag_id: number | null;
  sub_tag_id: number | null;
  notes: string;
  is_completed: boolean;
  completed_at: string | null;
  sort_order: number;
}

export interface TimelineItem {
  time: string;
  path: string;
  isNextDay?: boolean;
  logical_minute?: number;
}

export interface Toast {
  message: string;
  type: "success" | "error";
  visible: boolean;
}

export interface StorageHealth {
  ok: boolean;
  save_path_exists: boolean;
  save_path_writable: boolean;
  db_parent_exists: boolean;
  db_parent_writable: boolean;
  db_file_exists: boolean;
  db_file_writable: boolean;
  issues: string[];
}
