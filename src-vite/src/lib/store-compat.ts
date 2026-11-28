// Minimal svelte/store-compatible reactive core backed by Vue reactivity.
// Lets the framework-agnostic lib/state modules port verbatim; new code
// should prefer Pinia stores (src-vite/src/stores).
import { computed, ref, watch, type Ref } from "vue";

export interface Readable<T> {
  subscribe(fn: (v: T) => void): () => void;
}
export interface Writable<T> extends Readable<T> {
  set(v: T): void;
  update(fn: (v: T) => T): void;
}

export function writable<T>(initial: T): Writable<T> {
  const r = ref(initial) as Ref<T>;
  return {
    subscribe(fn: (v: T) => void) {
      fn(r.value);
      return watch(r, (nv) => fn(nv), { deep: true });
    },
    set(nv: T) {
      r.value = nv;
    },
    update(fn: (v: T) => T) {
      r.value = fn(r.value);
    },
  };
}

export function derived<S extends Readable<unknown>[], T>(
  stores: [...S],
  fn: (...args: { [K in keyof S]: S[K] extends Readable<infer U> ? U : never }) => T,
): Readable<T> {
  const c = computed(() => fn(...(stores.map((s) => get(s)) as never)));
  return {
    subscribe(fn: (v: T) => void) {
      fn(c.value);
      return watch(c, (nv) => fn(nv));
    },
  };
}

export function get<T>(store: Readable<T>): T {
  let v!: T;
  store.subscribe((x) => (v = x))();
  return v;
}

// App-level singleton modules call onMount at import time; in the Vue app
// there is no component lifecycle at module scope, so run immediately.
export function onMount(fn: () => void): void {
  if (typeof window !== "undefined") {
    fn();
  }
}
