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
