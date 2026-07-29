import { createApp } from "vue";
import App from "./App.vue";
import "./styles/tech.css";
import "./styles/reset.css";
import "./styles/label-clarity.css";
import "./styles/effects.css";

const app = createApp(App);
app.config.errorHandler = (error) => console.error("界面异常：", error);
app.mount("#app");
