// 生成一个随机且唯一的 ID
export function generateId(): string {
    if (typeof crypto !== 'undefined' && crypto.randomUUID) {
        // '93a0c191-7b89-47c1-a418-c1c1966c3d8b'
        return crypto.randomUUID();
    } else {
        // 'mpxsthft-osfeclvb'
        // return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;

        // 'e7a03200-ffee-44b3-bc8d-e3287580db1d'
        // 定义 UUID v4 的标准模板
        // '4' 代表版本号 (v4)
        // 'x' 代表随机 16 进制字符
        // 'y' 代表变体 (variant)，必须是 8, 9, a, 或 b
        // 替换模板中的 x 和 y
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
            // 生成 0 到 15 之间的随机整数 (0-f)
            // Math.random() * 16 得到 0~15.999... 
            // | 0 (按位或 0) 是一种快速取整的技巧，相当于 Math.floor()
            const r = (Math.random() * 16) | 0;
            // 根据 UUID 规范计算当前字符的值
            // 如果占位符是 'x'，直接用随机数 r
            // 如果占位符是 'y'，规范要求其二进制形式必须是 10xx (即十进制的 8, 9, 10, 11)
            // (r & 0x3) 保留最低两位，| 0x8 (二进制 1000) 强制将最高两位置为 10
            const v = c === 'x' ? r : (r & 0x3) | 0x8;
            // 将数字转为 16 进制字符串 (0-9, a-f)
            return v.toString(16);
        });
    }
}