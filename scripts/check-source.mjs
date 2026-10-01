import { execFileSync } from 'node:child_process';
import fs from 'node:fs';

const files=execFileSync('git',['ls-files','-z'],{encoding:'utf8'}).split('\0').filter(Boolean);
const forbidden=/(^|\/)(node_modules|target|dist|artifacts|\.build-cache|\.codex|\.agents)(\/|$)|(^|\/)\.env(?:\.|$)|\.(db|sqlite|sqlite3|scbackup|pfx|p12|key|log|tsbuildinfo)$/i;
const secrets=[/gh[pousr]_[A-Za-z0-9]{36,}/,/github_pat_[A-Za-z0-9_]{60,}/,/AKIA[A-Z0-9]{16}/,/sk-(?:proj-|ant-)[A-Za-z0-9_-]{40,}/,/-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----[\r\n]+[A-Za-z0-9+/=\r\n]{80,}/,/(?:C|D):[\\/](?:Users|Projekty AI)[\\/]/i];
const errors=[];
for(const file of files){
  if(forbidden.test(file)){errors.push(`${file}: local/generated/private file is tracked`);continue;}
  const stat=fs.lstatSync(file);
  if(stat.isSymbolicLink()){errors.push(`${file}: symlinks are not allowed in the release source set`);continue;}
  if(/\.(png|ico)$/i.test(file)){
    if(!file.startsWith('src-tauri/icons/'))errors.push(`${file}: image outside application icons`);
    continue;
  }
  if(stat.size>4*1024*1024){errors.push(`${file}: unexpected large source file`);continue;}
  const text=fs.readFileSync(file,'utf8');
  if(secrets.some(pattern=>pattern.test(text)))errors.push(`${file}: possible credential or local personal path; inspect locally`);
}
if(errors.length){process.stderr.write(errors.join('\n')+'\n');process.exit(1);}
process.stdout.write(`Checked ${files.length} tracked files: no blocked data or credential patterns found. This is a safeguard, not proof that no secret exists.\n`);
