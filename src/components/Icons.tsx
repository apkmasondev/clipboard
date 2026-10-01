import {
  FileText,
  Code2,
  Braces,
  Link,
  Folder,
  Image,
  Palette,
  Files,
  AlignLeft,
} from 'lucide-react';
import type { Kind } from '../types';
const icons = {
  text: AlignLeft,
  rich: FileText,
  code: Code2,
  json: Braces,
  link: Link,
  path: Folder,
  image: Image,
  color: Palette,
  files: Files,
};
export function KindIcon({ kind }: { kind: Kind }) {
  const Icon = icons[kind];
  return (
    <span className={`kind-icon kind-${kind}`}>
      <Icon size={18} />
    </span>
  );
}
