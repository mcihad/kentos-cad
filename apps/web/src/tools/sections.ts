import { TOOL_GROUP_LABEL, TOOL_SECTIONS, type ToolDescriptor, type ToolGroup } from './Tool';

/** One sub-heading of a tool group and its tools, in catalog order. */
export interface ToolSectionList {
  /** `group` or `group/section`, as menus name it (`@tools:draw/curve`). */
  readonly id: string;
  readonly label: string;
  readonly tools: readonly ToolDescriptor[];
}

/**
 * A group's tools split into its sections, in TOOL_SECTIONS order. Tools
 * without a known section follow under the group's name: adding a tool to
 * the catalog is enough for it to appear in every menu and in the ribbon.
 */
export function toolSections(tools: readonly ToolDescriptor[], group: ToolGroup): ToolSectionList[] {
  const inGroup = tools.filter((t) => t.group === group);
  const known: Record<string, string> = group in TOOL_SECTIONS ? TOOL_SECTIONS[group as keyof typeof TOOL_SECTIONS] : {};
  const out: ToolSectionList[] = Object.entries(known).map(([id, label]) => ({
    id: `${group}/${id}`,
    label,
    tools: inGroup.filter((t) => t.section === id),
  }));
  const rest = inGroup.filter((t) => !t.section || !(t.section in known));
  if (rest.length) out.push({ id: group, label: TOOL_GROUP_LABEL[group], tools: rest });
  return out.filter((s) => s.tools.length);
}

/** `draw` or `draw/curve` → the group and, when named, the one section (null when the name is unknown). */
export function parseToolRef(ref: string): { group: ToolGroup; section?: string } | null {
  const [group, section] = ref.split('/');
  if (!(group in TOOL_GROUP_LABEL)) return null;
  return { group: group as ToolGroup, section };
}
