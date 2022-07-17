import { App } from "vue";

import { Button } from "./button/button.vue";

const install = (Vue: App) => {
    Vue.component("vue", Button);
};

export {
    Button
};

export default install;
