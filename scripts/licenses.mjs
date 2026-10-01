import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
const metadata=JSON.parse(execFileSync('cargo',['metadata','--manifest-path','src-tauri/Cargo.toml','--format-version','1','--locked','--filter-platform','x86_64-pc-windows-msvc'],{encoding:'utf8',maxBuffer:32*1024*1024}));
const resolved=new Set(metadata.resolve.nodes.map(node=>node.id));
const runtimeNames=['@tauri-apps/api','react','react-dom','scheduler','lucide-react'];
const packages=[...metadata.packages.filter(p=>p.source && resolved.has(p.id)).map(p=>({name:p.name,version:p.version,license:p.license,dir:path.dirname(p.manifest_path)})),...runtimeNames.map(name=>{const dir=path.join('node_modules',name);const p=JSON.parse(fs.readFileSync(path.join(dir,'package.json'),'utf8'));return {name,version:p.version,license:p.license,dir};})];
let output='SUPER CLIPBOARD — THIRD-PARTY NOTICES\n\nThis inventory includes runtime and build dependencies for Windows. Alternative licenses are reproduced as supplied by each project.\n';
for(const p of packages.sort((a,b)=>a.name.localeCompare(b.name))){output+=`\n${'='.repeat(70)}\n${p.name} ${p.version} — ${p.license||'See license files'}\n`;const files=fs.readdirSync(p.dir).filter(n=>/^(license|licence|copying|notice|unlicense)([._-]|$)/i.test(n));for(const n of files){const file=path.join(p.dir,n);if(fs.statSync(file).isFile())output+=`\n--- ${n} ---\n${fs.readFileSync(file,'utf8')}\n`;}}
fs.writeFileSync('THIRD_PARTY_NOTICES.txt',output);
process.stdout.write(`Wrote notices for ${packages.length} packages.\n`);
