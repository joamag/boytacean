<template>
    <div v-bind:class="'hello'" v-on:click="() => count++">Hello This {{ name }} {{ count}}!</div>
</template>

<style scoped>
.hello {
    cursor: pointer;
    user-select: none;
}
</style>

<script>
export const App = {
    data() {
        return {
            name: "Vue",
            count: 1
        };
    }
};

export default App;
</script>
