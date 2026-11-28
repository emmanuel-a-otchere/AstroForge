import { createApp } from "vue";
import { createPinia } from "pinia";
import piniaPluginPersistedstate from "pinia-plugin-persistedstate";
import VueLazyload from "vue-lazyload";
import "cally";
import App from "./App.vue";
import router from "./router";
import i18n from "./i18n";
import "./assets/main.css";

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);

const app = createApp(App);
app.use(pinia);
app.use(router);
app.use(i18n);
app.use(VueLazyload, { preload: 1.2 });
app.mount("#app");
