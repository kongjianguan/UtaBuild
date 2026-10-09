import { access } from 'node:fs/promises';
import { constants } from 'node:fs';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const [command, ...args] = process.argv.slice(2);
if (!['render', 'verify', 'editor', 'calibrate'].includes(command)) {
  throw new Error('请选择 render、verify、editor 或 calibrate 命令');
}
const python = fileURLToPath(new URL(
  process.platform === 'win32' ? '.venv/Scripts/python.exe' : '.venv/bin/python',
  import.meta.url,
));
await access(python, constants.X_OK);
const script = fileURLToPath(new URL(`${command}.py`, import.meta.url));
const child = spawn(python, [script, ...args], { stdio: 'inherit' });
const code = await new Promise((resolve, reject) => {
  child.once('error', reject);
  child.once('exit', (status, signal) => {
    if (signal) reject(new Error(`Python 进程被 ${signal} 终止`));
    else resolve(status);
  });
});
process.exitCode = code;
