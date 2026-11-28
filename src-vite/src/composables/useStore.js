import { onScopeDispose, ref } from "vue";
import { get } from "@/lib/store-compat";

export function useStore(store) {
  const r = ref(get(store));
  const unsub = store.subscribe((v) => {
    r.value = v;
  });
  onScopeDispose(unsub);
  return r;
}
