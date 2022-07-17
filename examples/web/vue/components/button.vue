<template>
    <div v-on:click="() => count++">Hello {{ name }} {{ count}}!</div>
</template>

<script>
export const Button = {
    data() {
        return {
            name: "Vue",
            count: 1
        };
    }
};

export default Button;
</script>
