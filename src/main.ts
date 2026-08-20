import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import TrayMenu from "./views/TrayMenu.vue";
import { i18n } from "./i18n";
import "./style.css";

// 托盘那个窗口用的是同一份前端，靠 hash 分流：它没有侧边栏也没有标题栏，
// 整个外壳都不要，所以在这里就分岔，不进 App.vue
const isTray = window.location.hash === "#tray";
// 那扇窗是透明的，圆角和阴影由组件自己画；body 留着底色的话四个角会出方块
if (isTray) document.documentElement.classList.add("tray");

const app = createApp(isTray ? TrayMenu : App);
app.use(createPinia());
app.use(i18n);
app.mount("#app");

document.addEventListener('contextmenu', (event) => {
  event.preventDefault();
});
