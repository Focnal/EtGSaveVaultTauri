export function formatSize(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.min(units.length - 1, Math.floor(Math.log2(bytes) / 10))
  const v = bytes / 2 ** (10 * i)
  return `${v >= 100 ? Math.round(v) : v.toFixed(1)} ${units[i]}`
}

const pad = (n: number): string => String(n).padStart(2, '0')

export function formatTime(ts: number): string {
  const d = new Date(ts)
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

/** 紧凑时间：同年显示 MM-DD HH:mm，跨年补上年份，避免旧备份丢失年份 */
export function formatCompactTime(ts: number): string {
  const d = new Date(ts)
  const now = new Date()
  const date = `${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
  if (d.getFullYear() !== now.getFullYear()) return `${d.getFullYear()}-${date}`
  return `${date} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

const BACKUP_NAME_INVALID = /[\\/:*?"<>|]/

/** 备份命名规则；返回 null 表示合法。与 Rust validate_backup_name 保持一致。 */
export function validateBackupName(name: string): string | null {
  if (!name.trim()) return '备份名称不能为空'
  if (name !== name.trim()) return '名称首尾不能有空格'
  if (BACKUP_NAME_INVALID.test(name)) return '名称不能包含 \\ / : * ? " < > | 字符'
  if (name === '.' || name === '..') return '非法的备份名称'
  if (name.endsWith('.')) return '名称不能以句点结尾'
  const stem = name.split('.')[0].toUpperCase()
  if (/^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$/.test(stem)) {
    return '该名称是 Windows 保留名称，无法用作目录名'
  }
  if ([...name].length > 100) return '备份名称过长（最多 100 个字符）'
  return null
}

export function timestampName(): string {
  const d = new Date()
  return `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}_${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`
}
