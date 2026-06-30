#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::needless_range_loop)]

use flate2::read::ZlibDecoder;
use std::io::Read;

/// 将十六进制字符串解码为字节向量。
///
/// 支持长度为偶数的十六进制字符串（如 "48656c6c6f" → b"Hello"），
/// 奇数长度或包含非法字符时返回 `None`。
fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// 对数据进行 zlib 解压，返回 UTF-8 字符串。
///
/// 内部使用 `flate2` 库的 `ZlibDecoder` 进行解压。
/// 数据长度小于 2 字节时返回 `None`。
fn zlib_decompress(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).ok()?;
    String::from_utf8(decompressed).ok()
}

/// DES 算法的 S 盒查找表（8 个 S 盒，每个 64 项）。
///
/// 这是 DES 核心非线性变换的替换表，每个 S 盒将 6 位输入映射为 4 位输出，
/// 用于 `f` 函数中的混淆步骤。
const SBOX: [[u32; 64]; 8] = [
    [
        14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7, 0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12,
        11, 9, 5, 3, 8, 4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0, 15, 12, 8, 2, 4, 9,
        1, 7, 5, 11, 3, 14, 10, 0, 6, 13,
    ],
    [
        15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10, 3, 13, 4, 7, 15, 2, 8, 15, 12, 0, 1,
        10, 6, 9, 11, 5, 0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15, 13, 8, 10, 1, 3, 15,
        4, 2, 11, 6, 7, 12, 0, 5, 14, 9,
    ],
    [
        10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8, 13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5,
        14, 12, 11, 15, 1, 13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7, 1, 10, 13, 0, 6,
        9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12,
    ],
    [
        7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15, 13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2,
        12, 1, 10, 14, 9, 10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4, 3, 15, 0, 6, 10,
        10, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14,
    ],
    [
        2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9, 14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15,
        10, 3, 9, 8, 6, 4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14, 11, 8, 12, 7, 1, 14,
        2, 13, 6, 15, 0, 9, 10, 4, 5, 3,
    ],
    [
        12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11, 10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13,
        14, 0, 11, 3, 8, 9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6, 4, 3, 2, 12, 9, 5,
        15, 10, 11, 14, 1, 7, 6, 0, 8, 13,
    ],
    [
        4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1, 13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5,
        12, 2, 15, 8, 6, 1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2, 6, 11, 13, 8, 1, 4,
        10, 7, 9, 5, 0, 15, 14, 2, 3, 12,
    ],
    [
        13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7, 1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6,
        11, 0, 14, 9, 2, 7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8, 2, 1, 14, 7, 4, 10,
        8, 13, 15, 12, 9, 0, 3, 5, 6, 11,
    ],
];

/// 3DES 解密密钥（24 字节 = 3 个 8 字节 DES 密钥）。
///
/// QQ 音乐 QRC 格式使用的固定解密密钥。
const DECRYPT_KEY: &[u8; 24] = b"!@#)(*$%123ZXC!@!@#)(NHL";

/// 解密 QQ 音乐 QRC 歌词。
///
/// 接收 QRC 歌词数据的十六进制字符串表示，依次执行：
/// 1. 十六进制解码
/// 2. 3DES EDE 模式解密（使用固定密钥 `DECRYPT_KEY`）
/// 3. Zlib 解压
///
/// 返回解密后的 UTF-8 歌词字符串，任何步骤失败则返回 `None`。
#[must_use]
pub fn decrypt_qm_lyrics(hex: &str) -> Option<String> {
    let raw = hex_decode(hex)?;
    let decrypted = triple_des_crypt_ede(&raw, DECRYPT_KEY, false)?;
    zlib_decompress(&decrypted)
}

/// 从字节数组中提取指定位置的比特位。
///
/// `data` 为字节数组，`b` 为以比特为单位的偏移量，
/// `c` 控制结果左移的位数（用于拼接到目标位置）。
/// 以大端序方式处理比特位索引。
fn bitnum(data: &[u8], b: usize, c: u32) -> u32 {
    let byte_index = (b / 32) * 4 + 3 - (b % 32) / 8;
    if byte_index >= data.len() {
        return 0;
    }
    let byte_val = data[byte_index];
    let bit_val = (byte_val >> (7 - (b % 8))) & 1;
    (bit_val as u32) << c
}

/// 从 `u32` 值的第 `b` 位提取比特并右对齐位移。
///
/// 等价于：取 `a` 的第 `(31 - b)` 位，结果左移 `c` 位。
/// 用于 DES 的比特置换操作（右半部分取位）。
fn bitnum_intr(a: u32, b: usize, c: u32) -> u32 {
    ((a >> (31 - b)) & 1) << c
}

/// 从 `u32` 值的第 `b` 位提取比特并左对齐位移。
///
/// 将 `a` 左移 `b` 位后截取最高位，结果右移 `c` 位。
/// 用于 DES 的比特置换操作（左半部分取位）。
fn bitnum_intl(a: u32, b: usize, c: u32) -> u32 {
    let shifted = a << b;
    let masked = shifted & 0x8000_0000;
    masked >> c
}

/// 根据输入值计算 S 盒索引。
///
/// 输入 `a` 为 6 位有效值（低 6 位），
/// 输出为 0~63 的 S 盒索引，映射规则：
/// - 第 0、5 位组合作为行选择
/// - 第 1~4 位作为列选择
fn sbox_bit(a: u32) -> usize {
    ((a & 32) | ((a & 31) >> 1) | ((a & 1) << 4)) as usize
}

/// DES 初始置换（IP）。
///
/// 将 8 字节输入按照 IP 表重新排列比特位，
/// 输出分为左半部分 `s0` 和右半部分 `s1`（各 32 位）。
fn initial_permutation(input_data: &[u8; 8]) -> (u32, u32) {
    let s0_idx = [
        57, 49, 41, 33, 25, 17, 9, 1, 59, 51, 43, 35, 27, 19, 11, 3, 61, 53, 45, 37, 29, 21, 13, 5,
        63, 55, 47, 39, 31, 23, 15, 7,
    ];
    let s1_idx = [
        56, 48, 40, 32, 24, 16, 8, 0, 58, 50, 42, 34, 26, 18, 10, 2, 60, 52, 44, 36, 28, 20, 12, 4,
        62, 54, 46, 38, 30, 22, 14, 6,
    ];
    let s0: u32 = (0..32)
        .map(|i| bitnum(input_data, s0_idx[i], 31 - i as u32))
        .sum();
    let s1: u32 = (0..32)
        .map(|i| bitnum(input_data, s1_idx[i], 31 - i as u32))
        .sum();
    (s0, s1)
}

/// DES 逆初始置换（IP⁻¹）。
///
/// 将 `s0`（左 32 位）和 `s1`（右 32 位）合并，
/// 按 IP⁻¹ 表重新排列后输出 8 字节。
///
/// 输出字节顺序为：data[3], data[2], data[1], data[0],
/// data[7], data[6], data[5], data[4]。
fn inverse_permutation(s0: u32, s1: u32) -> [u8; 8] {
    let mut data = [0u8; 8];
    data[3] = ((bitnum_intr(s1, 7, 7)
        | bitnum_intr(s0, 7, 6)
        | bitnum_intr(s1, 15, 5)
        | bitnum_intr(s0, 15, 4)
        | bitnum_intr(s1, 23, 3)
        | bitnum_intr(s0, 23, 2)
        | bitnum_intr(s1, 31, 1)
        | bitnum_intr(s0, 31, 0))
        & 0xFF) as u8;
    data[2] = ((bitnum_intr(s1, 6, 7)
        | bitnum_intr(s0, 6, 6)
        | bitnum_intr(s1, 14, 5)
        | bitnum_intr(s0, 14, 4)
        | bitnum_intr(s1, 22, 3)
        | bitnum_intr(s0, 22, 2)
        | bitnum_intr(s1, 30, 1)
        | bitnum_intr(s0, 30, 0))
        & 0xFF) as u8;
    data[1] = ((bitnum_intr(s1, 5, 7)
        | bitnum_intr(s0, 5, 6)
        | bitnum_intr(s1, 13, 5)
        | bitnum_intr(s0, 13, 4)
        | bitnum_intr(s1, 21, 3)
        | bitnum_intr(s0, 21, 2)
        | bitnum_intr(s1, 29, 1)
        | bitnum_intr(s0, 29, 0))
        & 0xFF) as u8;
    data[0] = ((bitnum_intr(s1, 4, 7)
        | bitnum_intr(s0, 4, 6)
        | bitnum_intr(s1, 12, 5)
        | bitnum_intr(s0, 12, 4)
        | bitnum_intr(s1, 20, 3)
        | bitnum_intr(s0, 20, 2)
        | bitnum_intr(s1, 28, 1)
        | bitnum_intr(s0, 28, 0))
        & 0xFF) as u8;
    data[7] = ((bitnum_intr(s1, 3, 7)
        | bitnum_intr(s0, 3, 6)
        | bitnum_intr(s1, 11, 5)
        | bitnum_intr(s0, 11, 4)
        | bitnum_intr(s1, 19, 3)
        | bitnum_intr(s0, 19, 2)
        | bitnum_intr(s1, 27, 1)
        | bitnum_intr(s0, 27, 0))
        & 0xFF) as u8;
    data[6] = ((bitnum_intr(s1, 2, 7)
        | bitnum_intr(s0, 2, 6)
        | bitnum_intr(s1, 10, 5)
        | bitnum_intr(s0, 10, 4)
        | bitnum_intr(s1, 18, 3)
        | bitnum_intr(s0, 18, 2)
        | bitnum_intr(s1, 26, 1)
        | bitnum_intr(s0, 26, 0))
        & 0xFF) as u8;
    data[5] = ((bitnum_intr(s1, 1, 7)
        | bitnum_intr(s0, 1, 6)
        | bitnum_intr(s1, 9, 5)
        | bitnum_intr(s0, 9, 4)
        | bitnum_intr(s1, 17, 3)
        | bitnum_intr(s0, 17, 2)
        | bitnum_intr(s1, 25, 1)
        | bitnum_intr(s0, 25, 0))
        & 0xFF) as u8;
    data[4] = ((bitnum_intr(s1, 0, 7)
        | bitnum_intr(s0, 0, 6)
        | bitnum_intr(s1, 8, 5)
        | bitnum_intr(s0, 8, 4)
        | bitnum_intr(s1, 16, 3)
        | bitnum_intr(s0, 16, 2)
        | bitnum_intr(s1, 24, 1)
        | bitnum_intr(s0, 24, 0))
        & 0xFF) as u8;
    data
}

/// DES 轮函数 f。
///
/// 对 32 位输入 `state` 执行：
/// 1. 扩展置换（E 盒）：将 32 位扩展到 48 位
/// 2. 与轮密钥 `key`（6 字节/48 位）异或
/// 3. S 盒替换：8 个 S 盒将 48 位压缩回 32 位
/// 4. P 盒置换：32 位重新排列后输出
fn f(state: u32, key: &[u8; 6]) -> u32 {
    let s = state;

    let t1 = bitnum_intl(s, 31, 0)
        | ((s & 0xF000_0000) >> 1)
        | bitnum_intl(s, 4, 5)
        | bitnum_intl(s, 3, 6)
        | ((s & 0x0F00_0000) >> 3)
        | bitnum_intl(s, 8, 11)
        | bitnum_intl(s, 7, 12)
        | ((s & 0x00F0_0000) >> 5)
        | bitnum_intl(s, 12, 17)
        | bitnum_intl(s, 11, 18)
        | ((s & 0x000F_0000) >> 7)
        | bitnum_intl(s, 16, 23);

    let t2 = bitnum_intl(s, 15, 0)
        | ((s & 0x0000_F000) << 15)
        | bitnum_intl(s, 20, 5)
        | bitnum_intl(s, 19, 6)
        | ((s & 0x0000_0F00) << 13)
        | bitnum_intl(s, 24, 11)
        | bitnum_intl(s, 23, 12)
        | ((s & 0x0000_00F0) << 11)
        | bitnum_intl(s, 28, 17)
        | bitnum_intl(s, 27, 18)
        | ((s & 0x0000_000F) << 9)
        | bitnum_intl(s, 0, 23);

    let mut lrg = [
        ((t1 >> 24) & 0xFF) as u8,
        ((t1 >> 16) & 0xFF) as u8,
        ((t1 >> 8) & 0xFF) as u8,
        ((t2 >> 24) & 0xFF) as u8,
        ((t2 >> 16) & 0xFF) as u8,
        ((t2 >> 8) & 0xFF) as u8,
    ];

    for i in 0..6 {
        lrg[i] ^= key[i];
    }

    let res = (SBOX[0][sbox_bit((lrg[0] >> 2) as u32)] << 28)
        | (SBOX[1][sbox_bit((((lrg[0] & 3) << 4) | (lrg[1] >> 4)) as u32)] << 24)
        | (SBOX[2][sbox_bit((((lrg[1] & 0x0F) << 2) | (lrg[2] >> 6)) as u32)] << 20)
        | (SBOX[3][sbox_bit((lrg[2] & 0x3F) as u32)] << 16)
        | (SBOX[4][sbox_bit((lrg[3] >> 2) as u32)] << 12)
        | (SBOX[5][sbox_bit((((lrg[3] & 3) << 4) | (lrg[4] >> 4)) as u32)] << 8)
        | (SBOX[6][sbox_bit((((lrg[4] & 0x0F) << 2) | (lrg[5] >> 6)) as u32)] << 4)
        | SBOX[7][sbox_bit((lrg[5] & 0x3F) as u32)];

    let p = [
        15, 6, 19, 20, 28, 11, 27, 16, 0, 14, 22, 25, 4, 17, 30, 9, 1, 7, 23, 13, 31, 26, 2, 8, 18,
        12, 29, 5, 21, 10, 3, 24,
    ];
    let mut result: u32 = 0;
    for (i, &p_i) in p.iter().enumerate() {
        result |= bitnum_intl(res, p_i, i as u32);
    }
    result
}

/// 使用给定轮密钥调度表加密/解密一个 8 字节块。
///
/// 执行 16 轮 Feistel 网络结构：
/// - 前 15 轮：`s1 = f(s1, key[i]) ^ s0`，然后交换 `s0` 和 `s1`
/// - 第 16 轮：`s0 ^= f(s1, key[15])`（不交换）
/// - 最后执行逆初始置换
fn crypt_block(input_data: &[u8; 8], key: &[[u8; 6]; 16]) -> [u8; 8] {
    let (mut s0, mut s1) = initial_permutation(input_data);

    for idx in 0..15 {
        let prev_s1 = s1;
        s1 = f(s1, &key[idx]) ^ s0;
        s0 = prev_s1;
    }
    s0 ^= f(s1, &key[15]);

    inverse_permutation(s0, s1)
}

/// 生成 DES 轮密钥调度表（16 轮，每轮 6 字节）。
///
/// 过程：
/// 1. 从 8 字节密钥中提取 56 位（PC-1 置换），分为 C、D 各 28 位
/// 2. 每轮循环左移指定位数（1 或 2 位，由 `shifts` 表决定）
/// 3. 从 C、D 中选取 48 位（PC-2 压缩置换）作为该轮密钥
///
/// `mode` 参数控制密钥顺序：
/// - `mode = 0`：逆序（用于解密）
/// - `mode = 1`：正序（用于加密）
fn key_schedule(k: &[u8], mode: u32) -> [[u8; 6]; 16] {
    let shifts = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];
    let pc = [
        56, 48, 40, 32, 24, 16, 8, 0, 57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18, 10, 2,
        59, 51, 43, 35,
    ];
    let pd = [
        62, 54, 46, 38, 30, 22, 14, 6, 61, 53, 45, 37, 29, 21, 13, 5, 60, 52, 44, 36, 28, 20, 12,
        4, 27, 19, 11, 3,
    ];
    let comp = [
        13, 16, 10, 23, 0, 4, 2, 27, 14, 5, 20, 9, 22, 18, 11, 3, 25, 7, 15, 6, 26, 19, 12, 1, 40,
        51, 30, 36, 46, 54, 29, 39, 50, 44, 32, 47, 43, 48, 38, 55, 33, 52, 45, 41, 49, 35, 28, 31,
    ];

    let mut c: u32 = 0;
    let mut d: u32 = 0;
    for i in 0..28 {
        c |= bitnum(k, pc[i], 31 - i as u32);
        d |= bitnum(k, pd[i], 31 - i as u32);
    }

    let mut sched = [[0u8; 6]; 16];

    for i in 0..16 {
        c = ((c << shifts[i]) | (c >> (28 - shifts[i]))) & 0xFFFF_FFF0;
        d = ((d << shifts[i]) | (d >> (28 - shifts[i]))) & 0xFFFF_FFF0;

        let togen = if mode == 0 { 15 - i } else { i };

        for j in 0..24 {
            sched[togen][j / 8] |= (bitnum_intr(c, comp[j], 7 - (j % 8) as u32) & 0xFF) as u8;
        }
        for j in 24..48 {
            sched[togen][j / 8] |= (bitnum_intr(d, comp[j] - 27, 7 - (j % 8) as u32) & 0xFF) as u8;
        }
    }
    sched
}

/// 3DES EDE（Encrypt-Decrypt-Encrypt）模式加密/解密。
///
/// 使用三个独立的 DES 密钥（共 24 字节）：
/// - 加密时：密钥 1 加密 → 密钥 2 解密 → 密钥 3 加密
/// - 解密时：密钥 3 解密 → 密钥 2 加密 → 密钥 1 解密
///
/// 数据长度必须是 8 的倍数，否则返回 `None`。
fn triple_des_crypt_ede(data: &[u8], key: &[u8; 24], encrypt: bool) -> Option<Vec<u8>> {
    let schedules = if encrypt {
        [
            key_schedule(&key[0..8], 1),
            key_schedule(&key[8..16], 0),
            key_schedule(&key[16..24], 1),
        ]
    } else {
        [
            key_schedule(&key[16..24], 0),
            key_schedule(&key[8..16], 1),
            key_schedule(&key[0..8], 0),
        ]
    };

    if data.len() % 8 != 0 {
        return None;
    }
    let mut result = vec![0u8; data.len()];
    for i in (0..data.len()).step_by(8) {
        let mut block: [u8; 8] = {
            let slice: &[u8] = &data[i..i + 8];
            // SAFETY: i is checked to be at least 8 bytes from data.len()
            let arr: [u8; 8] = slice.try_into().unwrap();
            arr
        };
        for schedule in &schedules {
            block = crypt_block(&block, schedule);
        }
        result[i..i + 8].copy_from_slice(&block);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_decode_roundtrip() {
        let hex = "48656c6c6f";
        let bytes = hex_decode(hex).unwrap();
        assert_eq!(bytes, b"Hello");
    }

    #[test]
    fn hex_decode_invalid_returns_none() {
        assert!(hex_decode("ZZ").is_none());
        assert!(hex_decode("ABC").is_none());
    }

    #[test]
    fn zlib_decompress_detects_valid_data() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let data = b"hello world";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();

        let result = zlib_decompress(&compressed);
        assert_eq!(result.as_deref(), Some("hello world"));
    }

    #[test]
    fn decrypt_qm_invalid_input_graceful() {
        let result = decrypt_qm_lyrics("00");
        assert!(result.is_none());
    }

    #[test]
    fn decrypt_non_aligned_length_returns_none() {
        // 3 bytes (not 8-aligned) should fail at 3DES level
        let result = decrypt_qm_lyrics("000000");
        assert!(result.is_none());
    }

    #[test]
    fn decrypt_non_hex_returns_none() {
        assert!(decrypt_qm_lyrics("ZZZZZZZZZZZZZZZZ").is_none());
        assert!(decrypt_qm_lyrics("ABC").is_none()); // odd length
    }
}
