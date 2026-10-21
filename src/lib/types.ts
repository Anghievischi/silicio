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
  temperature_source: "sensor" | "acpi" | null;
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
