<template>
    <div v-bind:class="'hello'" v-on:click="() => count++">Hello {{ name }} {{ count}}!</div>
</template>

<style scoped>
.hello {
    cursor: pointer;
    user-select: none;
}
</style>

<script lang="ts">
export const Boytacean = {
    data() {
        return {
            name: "Vue",
            count: 1
        };
    }
};

export default Boytacean;
</script>
