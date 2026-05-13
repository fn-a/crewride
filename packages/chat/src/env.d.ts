/// <reference types="vite/client" />

// 告诉 TS .css 文件是合法模块
declare module '*.css';

declare module '~icons/*' {
    const svg: string;
    export default svg;
}
