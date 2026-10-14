#!/usr/bin/env node
/**
 * 版本号集中管理脚本。
 *
 * 单一版本源 = package.json 的 "version"，其余位置全部由它派生：
 *   - src-tauri/tauri.conf.json 的 version 写成 "../package.json"（Tauri 原生支持读取
 *     package.json 的 version 字段），因此不需要同步，本脚本只校验它没被改回硬编码。
 *   - src-tauri/Cargo.toml 的 [package] version 无法引用外部文件，只能由本脚本写入。
 *   - src-tauri/Cargo.lock 中本地包的版本条目由 cargo 自己刷新（本脚本尽力触发一次）。
 *
 * 用法：
 *   node scripts/sync-version.mjs          # 把 package.json 的版本同步进 Cargo.toml
 *   node scripts/sync-version.mjs --check  # 只校验是否一致，不一致退出码 1（CI 用）
 *
 * 平时用 `npm version patch|minor|major|<x.y.z>` 即可：package.json 的 "version"
 * 生命周期钩子会在改完 package.json 之后自动调用本脚本，并把改动一起提交。
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const pkgPath = join(root, 'package.json');
const confPath = join(root, 'src-tauri', 'tauri.conf.json');
const cargoPath = join(root, 'src-tauri', 'Cargo.toml');
const lockPath = join(root, 'src-tauri', 'Cargo.lock');
/** tauri.conf.json 里约定的指向：相对 src-tauri 的 package.json */
const confVersionPath = '../package.json';
const checkOnly = process.argv.includes('--check');

function fail(message) {
  console.error(`[sync-version] ${message}`);
  process.exit(1);
}

const version = JSON.parse(readFileSync(pkgPath, 'utf8')).version;
if (typeof version !== 'string' || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  fail(`package.json 的 "version"（${JSON.stringify(version)}）不是合法的 semver`);
}

// tauri.conf.json 必须保持指向 package.json，否则版本源就分裂成两个了
const conf = JSON.parse(readFileSync(confPath, 'utf8'));
if (conf.version !== confVersionPath) {
  fail(
    `src-tauri/tauri.conf.json 的 "version" 应为 "${confVersionPath}"，当前是 ${JSON.stringify(conf.version)}`,
  );
}

/**
 * 只替换 [package] 段内的 version 行，避免误改依赖版本或 [profile] 等其它段。
 * @returns {{ text: string, previous: string }}
 */
function withPackageVersion(toml, next) {
  const eol = toml.includes('\r\n') ? '\r\n' : '\n';
  const lines = toml.split(/\r?\n/);
  let inPackage = false;
  let previous = null;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (/^\s*\[/.test(line)) {
      inPackage = /^\s*\[package\]\s*$/.test(line);
      continue;
    }
    if (inPackage && previous === null && /^\s*version\s*=/.test(line)) {
      const match = /^(\s*version\s*=\s*)"([^"]*)"/.exec(line);
      if (!match) fail(`无法解析 src-tauri/Cargo.toml 中的 version 行：${line}`);
      previous = match[2];
      lines[i] = `${match[1]}"${next}"`;
    }
  }
  if (previous === null) fail('未在 src-tauri/Cargo.toml 的 [package] 段中找到 version 字段');
  return { text: lines.join(eol), previous };
}

const cargo = readFileSync(cargoPath, 'utf8');
const { text: updated, previous } = withPackageVersion(cargo, version);

if (updated === cargo) {
  console.log(`[sync-version] OK：package.json / tauri.conf.json / Cargo.toml 版本均为 ${version}`);
  process.exit(0);
}

if (checkOnly) {
  fail(
    `版本不一致：package.json 为 ${version}，src-tauri/Cargo.toml 为 ${previous}；` +
      '请运行 `npm run version:sync` 同步',
  );
}

writeFileSync(cargoPath, updated);
console.log(`[sync-version] src-tauri/Cargo.toml: ${previous} -> ${version}`);

// Cargo.lock 里本地包的版本条目由 cargo 维护，这里尽力触发一次刷新；
// 没有 cargo 时跳过，下次 cargo build 也会自动修正
if (existsSync(lockPath)) {
  const cargoBin = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
  const result = spawnSync(
    cargoBin,
    ['metadata', '--format-version', '1', '--manifest-path', join('src-tauri', 'Cargo.toml')],
    { cwd: root, stdio: 'ignore' },
  );
  if (result.error || result.status !== 0) {
    console.warn('[sync-version] 跳过 Cargo.lock 刷新（cargo 不可用），下次 cargo build 会自动补上');
  } else {
    console.log('[sync-version] 已刷新 src-tauri/Cargo.lock');
  }
}
