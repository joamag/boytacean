// eslint-disable-next-line no-undef
module.exports = {
    extends: ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
    parser: "@typescript-eslint/parser",
    plugins: ["@typescript-eslint"],
    rules: {
        "no-constant-condition": [
            "error",
            {
                checkLoops: false
            }
        ],
        "no-empty-function": "off",
        semi: ["error", "always"]
    },
    root: true
};
