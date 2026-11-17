// What a page is to the portal: given its address's parameters it fetches what it shows and answers with its place
// (the crumbs) and its body. `reload` draws it again after a step.
import type { Params } from '../app/router';
import type { Crumb } from '../shell/shell';

export type PageContext = { params: Params; query: URLSearchParams; reload: () => void };
/** `nav`: the sidebar item the page belongs to, when its address alone does not say (a dossier's list). */
export type Page = { crumbs: Crumb[]; body: HTMLElement; nav?: string };
export type PageFn = (ctx: PageContext) => Promise<Page>;
