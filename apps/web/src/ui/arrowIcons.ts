/**
 * The leader arrowheads' icons (docs/adr/0205 §7): each arrowhead drawn by itself from the rule's shapes, its tip at
 * the left. Written by scripts/ui/arrow_icons.py (scripts/fixtures/leader_cases.py's shapes); do not edit by hand.
 */
export const ARROW_ICONS = {
  leaderArrowFilled: '<path d="M11.5 10H17.5"/><path d="M2.5 10L11.5 8.5L11.5 11.5Z" fill="currentColor" stroke="none"/>',
  leaderArrowClosed: '<path d="M11.5 10H17.5"/><path d="M2.5 10L11.5 8.5L11.5 11.5Z"/>',
  leaderArrowOpen: '<path d="M2.5 10H17.5"/><path d="M11.5 8.5L2.5 10L11.5 11.5"/>',
  leaderArrowOpen30: '<path d="M2.5 10H17.5"/><path d="M11.5 7.59L2.5 10L11.5 12.41"/>',
  leaderArrowOpen90: '<path d="M2.5 10H17.5"/><path d="M7 5.5L2.5 10L7 14.5"/>',
  leaderArrowDot: '<path d="M6 10H17.5"/><circle cx="3.75" cy="10" r="2.25" fill="currentColor" stroke="none"/>',
  leaderArrowDotSmall: '<path d="M3.75 10H17.5"/><circle cx="2.62" cy="10" r="1.12" fill="currentColor" stroke="none"/>',
  leaderArrowDotBlank: '<path d="M6 10H17.5"/><circle cx="3.75" cy="10" r="2.25"/>',
  leaderArrowOblique: '<path d="M6 10H17.5"/><path d="M1.5 14.5L10.5 5.5"/>',
  leaderArrowArchTick: '<path d="M6.4 10H17.5"/><path d="M2.3 14.9L11.3 5.9L10.5 5.1L1.5 14.1Z" fill="currentColor" stroke="none"/>',
  leaderArrowBoxFilled: '<path d="M6 10H17.5"/><path d="M1.5 12.25L6 12.25L6 7.75L1.5 7.75Z" fill="currentColor" stroke="none"/>',
  leaderArrowBoxBlank: '<path d="M6 10H17.5"/><path d="M1.5 12.25L6 12.25L6 7.75L1.5 7.75Z"/>',
  leaderArrowDatum: '<path d="M11.5 10H17.5"/><path d="M2.5 14.5L11.5 10L2.5 5.5Z" fill="currentColor" stroke="none"/>',
  leaderArrowNone: '<path d="M2.5 10H17.5"/>',
} as const;
