// Playbook Types - Screenpipe Automation Rules
// Auto-aligned with crates/screenpipe-core/src/playbook_engine.rs

// ─── Trigger Types ───────────────────────────────────────────────────────────

/**
 * Trigger when a specific application is opened or becomes active
 */
export interface AppOpenTrigger {
  type: "app_open";
  /** Name of the application to monitor (e.g., "zoom", "chrome", "code") */
  app_name: string;
  /** Optional: specific window title pattern to match */
  window_name?: string;
}

/**
 * Trigger based on a cron schedule
 */
export interface TimeTrigger {
  type: "time";
  /** Cron expression (e.g., "0 9 * * 1-5" for 9 AM weekdays) */
  cron: string;
  /** Human-readable description of the schedule */
  description?: string;
}

/**
 * Trigger when specific keywords are detected in screen content or audio
 */
export interface KeywordTrigger {
  type: "keyword";
  /** Pattern to search for (supports regex) */
  pattern: string;
  /** Where to search for the pattern */
  source: "ocr" | "audio" | "both";
  /** Minimum confidence threshold (0-1) */
  threshold?: number;
}

/**
 * Trigger when user enters a specific context (combination of conditions)
 */
export interface ContextTrigger {
  type: "context";
  /** Required apps to be open */
  apps?: string[];
  /** Required window titles */
  windows?: string[];
  /** Time range (e.g., "09:00-17:00") */
  time_range?: string;
  /** Days of week (0-6, 0 = Sunday) */
  days_of_week?: number[];
}

export type Trigger = AppOpenTrigger | TimeTrigger | KeywordTrigger | ContextTrigger;

// ─── Action Types ────────────────────────────────────────────────────────────

/**
 * Send a notification to the user
 */
export interface NotifyAction {
  type: "notify";
  /** Title of the notification */
  title: string;
  /** Message body */
  message: string;
  /** Optional actions the user can take */
  actions?: Array<{
    id: string;
    label: string;
  }>;
  /** Whether the notification persists until dismissed */
  persistent?: boolean;
}

/**
 * Generate a summary of recent activity
 */
export interface SummarizeAction {
  type: "summarize";
  /** Timeframe in minutes to summarize */
  timeframe: number;
  /** What to focus on in the summary */
  focus?: "all" | "action_items" | "decisions" | "key_points";
  /** Where to send the summary (notification, clipboard, etc.) */
  output?: "notification" | "clipboard" | "pipe";
}

/**
 * Enable or disable focus mode
 */
export interface FocusModeAction {
  type: "focus_mode";
  /** Whether to enable or disable focus mode */
  enabled: boolean;
  /** Duration in minutes (0 = indefinite) */
  duration?: number;
  /** Apps to allow during focus mode */
  allowed_apps?: string[];
  /** Whether to silence notifications */
  silence_notifications?: boolean;
}

/**
 * Run a specific pipe
 */
export interface RunPipeAction {
  type: "run_pipe";
  /** ID of the pipe to run */
  pipe_id: string;
  /** Optional parameters to pass to the pipe */
  params?: Record<string, unknown>;
}

/**
 * Tag content automatically
 */
export interface TagAction {
  type: "tag";
  /** Tags to apply */
  tags: string[];
  /** Timeframe in minutes to tag content from */
  timeframe: number;
}

/**
 * Execute a custom webhook
 */
export interface WebhookAction {
  type: "webhook";
  /** URL to call */
  url: string;
  /** HTTP method */
  method: "GET" | "POST" | "PUT" | "DELETE";
  /** Headers to include */
  headers?: Record<string, string>;
  /** Body payload */
  body?: Record<string, unknown>;
}

export type Action =
  | NotifyAction
  | SummarizeAction
  | FocusModeAction
  | RunPipeAction
  | TagAction
  | WebhookAction;

// ─── Playbook Definition ─────────────────────────────────────────────────────

/**
 * A playbook defines automated rules for Screenpipe
 */
export interface Playbook {
  /** Unique identifier */
  id: string;
  /** Display name */
  name: string;
  /** Description of what this playbook does */
  description?: string;
  /** Whether the playbook is currently active */
  enabled: boolean;
  /** Triggers that activate this playbook */
  triggers: Trigger[];
  /** Actions to execute when triggered */
  actions: Action[];
  /** Optional: cooldown between activations (in minutes) */
  cooldown_minutes?: number;
  /** Optional: maximum executions per day */
  max_executions_per_day?: number;
  /** When the playbook was created */
  created_at?: string;
  /** When the playbook was last updated */
  updated_at?: string;
  /** Whether this is a built-in playbook */
  is_builtin?: boolean;
  /** Icon for the playbook (emoji or URL) */
  icon?: string;
  /** Color for UI representation */
  color?: string;
}

// ─── Playbook Execution ──────────────────────────────────────────────────────

/**
 * Status of a playbook execution
 */
export interface PlaybookExecution {
  /** Unique execution ID */
  id: string;
  /** Playbook ID that was executed */
  playbook_id: string;
  /** When the execution started */
  started_at: string;
  /** When the execution completed (if finished) */
  completed_at?: string;
  /** Current status */
  status: "running" | "completed" | "failed" | "cancelled";
  /** Which trigger activated the playbook */
  triggered_by: Trigger;
  /** Results from each action */
  action_results: ActionResult[];
  /** Error message if failed */
  error?: string;
}

/**
 * Result of a single action execution
 */
export interface ActionResult {
  /** Action that was executed */
  action: Action;
  /** Whether the action succeeded */
  success: boolean;
  /** Result data or error message */
  result?: unknown;
  /** Error message if failed */
  error?: string;
  /** Execution time in milliseconds */
  duration_ms: number;
}

// ─── API Types ───────────────────────────────────────────────────────────────

/**
 * Request to create a new playbook
 */
export interface CreatePlaybookRequest {
  name: string;
  description?: string;
  triggers: Trigger[];
  actions: Action[];
  cooldown_minutes?: number;
  max_executions_per_day?: number;
  icon?: string;
  color?: string;
}

/**
 * Request to update an existing playbook
 */
export interface UpdatePlaybookRequest {
  name?: string;
  description?: string;
  enabled?: boolean;
  triggers?: Trigger[];
  actions?: Action[];
  cooldown_minutes?: number;
  max_executions_per_day?: number;
  icon?: string;
  color?: string;
}

/**
 * Response from listing playbooks
 */
export interface ListPlaybooksResponse {
  playbooks: Playbook[];
  total: number;
}

/**
 * Response from getting playbook executions
 */
export interface ListExecutionsResponse {
  executions: PlaybookExecution[];
  total: number;
}

// ─── Built-in Playbooks ──────────────────────────────────────────────────────

/**
 * Pre-defined playbook templates
 */
export const BUILTIN_PLAYBOOKS: Playbook[] = [
  {
    id: "daily-standup",
    name: "Daily Standup",
    description: "Automatically generate a summary of your work at 9 AM on weekdays",
    enabled: false,
    is_builtin: true,
    icon: "📅",
    color: "#3B82F6",
    triggers: [
      {
        type: "time",
        cron: "0 9 * * 1-5",
        description: "Every weekday at 9:00 AM",
      },
    ],
    actions: [
      {
        type: "summarize",
        timeframe: 1440, // 24 hours
        focus: "action_items",
        output: "notification",
      },
    ],
    cooldown_minutes: 60,
  },
  {
    id: "customer-call",
    name: "Customer Call",
    description: "Focus on action items when joining Zoom or Google Meet",
    enabled: false,
    is_builtin: true,
    icon: "🎥",
    color: "#10B981",
    triggers: [
      {
        type: "app_open",
        app_name: "zoom",
      },
      {
        type: "app_open",
        app_name: "chrome",
        window_name: "meet.google.com",
      },
    ],
    actions: [
      {
        type: "focus_mode",
        enabled: true,
        duration: 60,
        silence_notifications: true,
        allowed_apps: ["zoom", "chrome"],
      },
      {
        type: "notify",
        title: "Customer Call Mode",
        message: "Focus mode enabled. I'll summarize action items at the end of the call.",
        persistent: false,
      },
    ],
  },
  {
    id: "deep-work",
    name: "Deep Work",
    description: "Block distractions during focus time",
    enabled: false,
    is_builtin: true,
    icon: "🎯",
    color: "#8B5CF6",
    triggers: [
      {
        type: "context",
        time_range: "09:00-12:00",
        days_of_week: [1, 2, 3, 4, 5],
      },
    ],
    actions: [
      {
        type: "focus_mode",
        enabled: true,
        duration: 180,
        silence_notifications: true,
        allowed_apps: ["code", "cursor", "vscode", "terminal"],
      },
    ],
    cooldown_minutes: 240,
  },
];
