// Espelha as structs serializadas em src-tauri/src/hw/cpu.rs.

export interface Codename {
  name: string;
  microarch: string;
  process: string;
}

export interface Signature {
  family: number;
  model: number;
  stepping: number;
  base_family: number;
  base_model: number;
  ext_family: number;
  ext_model: number;
}

export interface ClockRange {
  min_mhz: number | null;
  max_mhz: number | null;
  driver: string | null;
  governor: string | null;
}

export interface Cache {
  level: number;
  kind: string;
  size_kib: number;
  ways: number;
  line_bytes: number;
  instances: number;
}

export interface Core {
  id: number;
  threads: number[];
  cluster: number;
  kind: "performance" | "efficiency" | null;
}

export interface InstructionGroup {
  name: string;
  items: string[];
}

export interface CpuInfo {
  brand: string;
  vendor: string;
  vendor_id: string;
  codename: Codename | null;
  signature: Signature;
  microcode: string | null;
  cores: number;
  threads: number;
  clocks: ClockRange;
  caches: Cache[];
  topology: Core[];
  instructions: InstructionGroup[];
}

export interface ThreadLive {
  mhz: number;
  usage: number;
}

export interface CpuLive {
  threads: ThreadLive[];
  avg_mhz: number;
  peak_mhz: number;
  usage: number;
  temperature_c: number | null;
  temperature_source: "sensor" | null;
}

export interface PcieLink {
  current_gen: number | null;
  current_width: number | null;
  max_gen: number | null;
  max_width: number | null;
}

export interface PciDevice {
  address: string;
  class: number;
  vendor_id: number;
  device_id: number;
  revision: number | null;
  vendor: string | null;
  name: string | null;
  link: PcieLink | null;
}

export interface BoardInfo {
  system: { vendor: string | null; product: string | null; family: string | null; chassis: string | null };
  board: { vendor: string | null; name: string | null; version: string | null };
  bios: { vendor: string | null; version: string | null; date: string | null; release: string | null };
  chipset: PciDevice | null;
  southbridge: PciDevice | null;
  graphics: PciDevice[];
  gpu_slot_gen: number | null;
  external_pcie: "thunderbolt" | "usb4" | null;
}

export interface Slot {
  locator: string | null;
  bank: string | null;
  size_bytes: number;
  kind: string | null;
  detail: string | null;
  form_factor: string | null;
  speed_mts: number | null;
  configured_mts: number | null;
  manufacturer: string | null;
  part_number: string | null;
  rank: number | null;
  data_width: number | null;
}

export interface MemoryInfo {
  total_bytes: number;
  max_capacity_bytes: number | null;
  kind: string | null;
  speed_mts: number | null;
  channels: number;
  slots: Slot[];
}

export interface MemoryLive {
  used_bytes: number;
  available_bytes: number;
  total_bytes: number;
  swap_used_bytes: number;
  swap_total_bytes: number;
}

export interface ThreadMsr {
  pstate_mhz: number | null;
  ratio: number | null;
  voltage: number | null;
}

export interface PrivilegedInfo {
  processor: {
    socket: string | null;
    voltage: number | null;
    external_clock_mhz: number | null;
    max_speed_mhz: number | null;
    current_speed_mhz: number | null;
  } | null;
  bios_vendor: string | null;
  bus_mhz: number | null;
  base_ratio: number | null;
  max_mhz: number | null;
  threads: ThreadMsr[];
  errors: string[];
}

export interface SpeedBin {
  speed_mts: number;
  clock_mhz: number;
  cl: number;
  trcd: number;
  trp: number;
  tras: number;
  trc: number;
}

/** SPD decodificado (DDR4 ou DDR5). */
export interface SpdDecoded {
  spd_revision: string;
  crc_ok: boolean;
  size_mb: number;
  max_speed_mts: number;
  pc_name: string;
  ranks: number;
  device_width: number;
  bus_width: number;
  die_density_gbit: number;
  bank_groups: number;
  banks_per_group: number;
  voltage: number | null;
  module_manufacturer: string | null;
  dram_manufacturer: string | null;
  part_number: string | null;
  manufactured: [number, number] | null;
  cas_supported: number[];
  timings_ns: { tck_min: number; taa: number; trcd: number; trp: number; tras: number; trc: number; trfc1: number };
  table: SpeedBin[];
  xmp: { version: string; profiles: MemProfile[] } | null;
  expo: MemProfile[] | null;
}

/** Perfil XMP ou AMD EXPO. */
export interface MemProfile {
  number: number;
  name: string | null;
  voltage: number;
  speed_mts: number;
  clock_mhz: number;
  cl: number;
  trcd: number;
  trp: number;
  tras: number;
  trc: number;
  trfc1: number;
}

export interface SpdModule {
  address: string;
  slot: number;
  kind: string;
  module_type: string | null;
  decoded: SpdDecoded | null;
}

export interface Gpu {
  id: string;
  name: string | null;
  vendor: string | null;
  integrated: boolean | null;
  driver: string | null;
  driver_version: string | null;
  vbios: string | null;
  vram_bytes: number | null;
  shared_bytes: number | null;
  max_core_mhz: number | null;
  max_memory_mhz: number | null;
  link: PcieLink | null;
}

export interface GpuLive {
  id: string;
  core_mhz: number | null;
  memory_mhz: number | null;
  usage: number | null;
  vram_used_bytes: number | null;
  temperature_c: number | null;
  power_w: number | null;
  voltage_mv: number | null;
  fan_rpm: number | null;
  fan_percent: number | null;
  limit: "thermal" | "power" | null;
}

export interface BenchResult {
  threads: number;
  seconds: number;
  units: number;
  score: number;
}

export interface StressStatus {
  running: boolean;
  seconds: number;
  units: number;
}

// Espelha src-tauri/src/hw/upgrade.rs.

export interface GpuSuggestion {
  name: string;
  lanes: number;
  vram_gb: number;
}

export interface Upgrades {
  form: "desktop" | "laptop" | "compact" | "unknown";
  top: "memory-dual-channel" | "memory-capacity" | "gpu" | "cpu" | "none";
  gpu: {
    path: "slot" | "external" | "none";
    slot_gen: number | null;
    external_port: "thunderbolt" | "usb4" | null;
    current: string | null;
    tier: "entry" | "mid" | "high";
    psu_watts: number;
    suggestions: GpuSuggestion[];
    narrow_slot: boolean;
    no_rebar: boolean;
  };
  memory: {
    action: "soldered" | "dual-channel" | "add" | "replace" | "maxed" | "unknown";
    kind: string | null;
    form_factor: string | null;
    speed_mts: number | null;
    installed_bytes: number;
    max_bytes: number | null;
    slots_total: number;
    slots_used: number;
    module_count: number;
    module_bytes: number;
    target_bytes: number;
  };
  cpu: {
    path: "soldered" | "socket" | "unknown";
    socket: string | null;
    ceiling: string[];
    platform_end: boolean;
    check_bios: boolean;
    at_ceiling: boolean;
  };
}

export interface SystemInfo {
  app_version: string;
  os: string | null;
  kernel: string | null;
  arch: string;
}

// Espelha src-tauri/src/hw/peripherals.rs e edid.rs.

export interface DisplayMode {
  width: number;
  height: number;
  refresh_hz: number;
}

export interface Display {
  connector: string | null;
  internal: boolean | null;
  vendor_code: string;
  vendor: string | null;
  name: string | null;
  product_code: number;
  year: number | null;
  week: number | null;
  version: string;
  width_cm: number | null;
  height_cm: number | null;
  diagonal_in: number | null;
  native: DisplayMode | null;
  modes: DisplayMode[];
  refresh_min: number | null;
  refresh_max: number | null;
  bit_depth: number | null;
  interface: string | null;
  hdr: boolean;
  bt2020: boolean;
}

export type DeviceKind =
  | "keyboard"
  | "mouse"
  | "hid"
  | "audio"
  | "camera"
  | "storage"
  | "smartcard"
  | "bluetooth"
  | "network"
  | "gamepad"
  | "receiver";

export interface Device {
  bus: "usb" | "bluetooth";
  vendor_id: number | null;
  product_id: number | null;
  vendor: string | null;
  name: string | null;
  kinds: DeviceKind[];
  speed_mbps: number | null;
  usb_version: string | null;
  max_power_ma: number | null;
  driver: string | null;
}

export interface Battery {
  name: string;
  capacity: number | null;
  status: string | null;
}

export interface AudioDevice {
  name: string;
  card: string | null;
  direction: "output" | "input";
  bus: "usb" | "hdmi" | "bluetooth" | "internal" | null;
  form: string | null;
  default: boolean;
  sample_rate: number | null;
  bits: number | null;
  channels: number | null;
}

export interface CameraFormat {
  fourcc: string;
  description: string;
  modes: { width: number; height: number; max_fps: number | null }[];
}

export interface Camera {
  name: string;
  bus: "usb" | null;
  formats: CameraFormat[];
}

export interface Peripherals {
  displays: Display[];
  devices: Device[];
  batteries: Battery[];
  audio: AudioDevice[];
  cameras: Camera[];
}

export interface DiskHealth {
  status: "good" | "warning" | "bad";
  life_left: number | null;
  written_bytes: number | null;
  read_bytes: number | null;
  power_on_hours: number | null;
  power_cycles: number | null;
  unsafe_shutdowns: number | null;
  media_errors: number | null;
  bad_sectors: number | null;
  available_spare: number | null;
  critical_warning: number | null;
  failing: boolean;
}

export interface Volume {
  mount_point: string;
  label: string | null;
  file_system: string | null;
  total_bytes: number;
  available_bytes: number;
}

export interface Disk {
  id: string;
  model: string | null;
  vendor: string | null;
  firmware: string | null;
  size_bytes: number;
  bus: "nvme" | "sata" | "usb" | "mmc" | "scsi" | "virtual" | null;
  media: "ssd" | "hdd" | null;
  removable: boolean;
  link: PcieLink | null;
  sata_speed: string | null;
  temperature_c: number | null;
  health: DiskHealth | null;
  volumes: Volume[];
}

export interface SystemBattery {
  model: string | null;
  manufacturer: string | null;
  technology: string | null;
  design_wh: number | null;
  full_wh: number | null;
  now_wh: number | null;
  percent: number | null;
  cycles: number | null;
  status: "charging" | "discharging" | "full" | "not_charging" | "unknown";
  power_w: number | null;
  voltage_v: number | null;
  ac_online: boolean | null;
}

// Espelha src-tauri/src/hw/diagnosis.rs.

export type FindingKind =
  | { code: "memory-single-channel" }
  | { code: "memory-profile-off"; profile: "xmp" | "expo"; current_mts: number; profile_mts: number }
  | { code: "memory-below-rated"; current_mts: number; rated_mts: number }
  | { code: "memory-mixed" }
  | { code: "memory-low"; installed_bytes: number }
  | { code: "cpu-powersave"; driver: string }
  | { code: "gpu-link-narrow"; current_width: number; max_width: number }
  | { code: "gpu-no-driver" }
  | { code: "gpu-nouveau" }
  | { code: "disk-failing" }
  | { code: "disk-caution"; life_left: number | null; bad_sectors: number | null }
  | { code: "disk-life"; life_left: number; hours_left: number }
  | { code: "disk-full"; mount_point: string; free_bytes: number; free_pct: number }
  | { code: "disk-hot"; temperature_c: number }
  | { code: "system-on-hdd"; mount_point: string }
  | { code: "disk-link-narrow"; current_width: number; max_width: number }
  | { code: "battery-worn"; health_pct: number; cycles: number | null }
  | { code: "bios-old"; years: number }
  | { code: "usb-slow-port"; speed_mbps: number };

export type Finding = FindingKind & {
  severity: "info" | "warning" | "problem";
  area: "memory" | "cpu" | "gpu" | "storage" | "battery" | "board" | "peripherals";
  subject: string | null;
};

/** Achado de um código específico, com os campos dele. */
export type FindingOf<C extends Finding["code"]> = Extract<Finding, { code: C }>;

export interface Diagnosis {
  status: "good" | "attention" | "problem";
  score: number;
  findings: Finding[];
}

// Espelha src-tauri/src/session.rs.

export interface SessionSample {
  t: number;
  cpu_usage: number;
  thread_max: number;
  cpu_mhz: number;
  cpu_temp: number | null;
  gpu_usage: number | null;
  gpu_mhz: number | null;
  gpu_temp: number | null;
  gpu_power: number | null;
  gpu_limit: "thermal" | "power" | null;
  vram_used: number | null;
  igpu_usage: number | null;
  ram_used: number;
  swap_used: number;
  on_battery: boolean | null;
}

export interface SessionStatus {
  running: boolean;
  seconds: number;
  samples: number;
  last: SessionSample | null;
  has_report: boolean;
}

type Throttle = {
  peak_mhz: number;
  low_mhz: number;
  seconds: number;
  first_at: number;
  cause: "heat" | "power" | "unknown";
  temp_max: number | null;
};

export type SessionFinding = { severity: "info" | "warning" | "problem" } & (
  | { code: "wrong-gpu" }
  | ({ code: "cpu-throttle" } & Throttle)
  | { code: "cpu-hot"; temp_max: number; seconds: number }
  | ({ code: "gpu-throttle"; reported: boolean } & Throttle)
  | { code: "gpu-thermal-limit"; seconds: number; temp_max: number | null }
  | { code: "gpu-hot"; temp_max: number; seconds: number }
  | { code: "vram-full"; peak_bytes: number; total_bytes: number }
  | { code: "ram-full"; peak_bytes: number; total_bytes: number; swap_growth_bytes: number }
  | { code: "on-battery"; seconds: number }
  | { code: "short-session"; load_seconds: number }
);

export type SessionFindingOf<C extends SessionFinding["code"]> = Extract<SessionFinding, { code: C }>;

export type SessionLimit = "gpu" | "cpu" | "vram" | "ram" | "thermal" | "capped";

export interface SessionStat {
  avg: number;
  max: number;
}

export interface SessionReport {
  seconds: number;
  load_seconds: number;
  verdict: SessionLimit | "balanced" | "idle";
  /** Parte do tempo de jogo em cada limite, da maior para a menor. */
  limits: { code: SessionLimit; pct: number }[];
  findings: SessionFinding[];
  stats: Record<
    "cpu_usage" | "thread_max" | "cpu_mhz" | "cpu_temp" | "gpu_usage" | "gpu_mhz" | "gpu_temp" | "gpu_power" | "vram_used" | "ram_used",
    SessionStat | null
  >;
  context: { gpu_name: string | null; dedicated: boolean; hybrid: boolean; vram_total: number | null; ram_total: number };
  samples: SessionSample[];
}
