import { App } from "vue";

import Components from "./components";

const install = (Vue: App) => {
    Vue.use(Components);
};

export * from "./components";

export default install;
