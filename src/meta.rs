
use std::fs;

#[derive(Debug, Clone)]
pub struct MethodRow {
    pub rva: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TypeDefRow {
    pub name: String,
    pub namespace: String,
    pub method_start: u32,
    pub method_end: u32,
    pub field_start: u32,
    pub field_end: u32,
}

#[derive(Debug, Clone)]
pub struct FieldRow {
    pub name: String,
}

pub struct CliImage {
    pub data: Vec<u8>,
    pub strings_base: usize,
    pub us_base: usize,
    pub us_size: usize,
    pub typedefs: Vec<TypeDefRow>,
    pub methods: Vec<MethodRow>,
    pub fields: Vec<FieldRow>,
    pub typerefs: Vec<String>,
    pub memberrefs: Vec<(u8, u32, String)>,
}

fn u16_at(d: &[u8], p: usize) -> u32 {
    u16::from_le_bytes([d[p], d[p + 1]]) as u32
}

fn u32_at(d: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]])
}

impl CliImage {
    pub fn load(path: &str) -> Result<CliImage, String> {
        let data = fs::read(path).map_err(|e| format!("read: {}", e))?;
        CliImage::parse(data)
    }

    pub fn parse(data: Vec<u8>) -> Result<CliImage, String> {
        if data.len() < 0x100 || &data[0..2] != b"MZ" {
            return Err("not PE".into());
        }
        let pe_off = u32_at(&data, 0x3C) as usize;
        if &data[pe_off..pe_off + 4] != b"PE\0\0" {
            return Err("bad PE sig".into());
        }
        let coff = pe_off + 4;
        let num_sections = u16_at(&data, coff + 2) as usize;
        let opt_size = u16_at(&data, coff + 16) as usize;
        let opt = coff + 20;
        let magic = u16_at(&data, opt);
        let dd_off = if magic == 0x20B { opt + 112 } else { opt + 96 };
        let clr_rva = u32_at(&data, dd_off + 14 * 8);
        let clr_size = u32_at(&data, dd_off + 14 * 8 + 4);
        if clr_rva == 0 || clr_size < 72 {
            return Err("not CLI image".into());
        }
        let sec_tab = opt + opt_size;
        let mut sections: Vec<(u32, u32, u32)> = Vec::new();
        for i in 0..num_sections {
            let s = sec_tab + i * 40;
            let vsize = u32_at(&data, s + 8);
            let va = u32_at(&data, s + 12);
            let raw = u32_at(&data, s + 20);
            sections.push((va, vsize, raw));
        }
        let rva_to_off = |rva: u32| -> Result<usize, String> {
            for &(va, vsize, raw) in &sections {
                if rva >= va && rva < va + vsize.max(1) {
                    return Ok((rva - va + raw) as usize);
                }
            }
            Err(format!("rva 0x{:X} not mapped", rva))
        };
        let clr_off = rva_to_off(clr_rva)?;
        let meta_rva = u32_at(&data, clr_off + 8);
        let meta_size = u32_at(&data, clr_off + 12) as usize;
        let meta_off = rva_to_off(meta_rva)?;
        let meta_end = meta_off + meta_size;
        if u32_at(&data, meta_off) != 0x424A5342 {
            return Err("bad meta sig".into());
        }
        let version_len = u32_at(&data, meta_off + 12) as usize;
        let mut p = meta_off + 16 + version_len;
        p += 2;
        let stream_count = u16_at(&data, p) as usize;
        p += 2;
        let mut strings_base = 0usize;
        let mut us_base = 0usize;
        let mut us_size = 0usize;
        let mut tilde_off = 0usize;
        let mut tilde_size = 0usize;
        for _ in 0..stream_count {
            let s_off = u32_at(&data, p) as usize;
            let s_size = u32_at(&data, p + 4) as usize;
            p += 8;
            let name_start = p;
            while data[p] != 0 {
                p += 1;
            }
            let name = String::from_utf8_lossy(&data[name_start..p]).to_string();
            p += 1;
            p = (p + 3) & !3;
            let abs = meta_off + s_off;
            if name == "#Strings" {
                strings_base = abs;
            } else if name == "#US" {
                us_base = abs;
                us_size = s_size;
            } else if name == "#~" || name == "#-" {
                tilde_off = abs;
                tilde_size = s_size;
            }
        }
        if tilde_off == 0 {
            return Err("no #~ stream".into());
        }
        let tilde_end = (tilde_off + tilde_size).min(meta_end);
        let _ = tilde_end;

        let heap_sizes = data[tilde_off + 6];
        let valid = u64::from_le_bytes([
            data[tilde_off + 8],
            data[tilde_off + 9],
            data[tilde_off + 10],
            data[tilde_off + 11],
            data[tilde_off + 12],
            data[tilde_off + 13],
            data[tilde_off + 14],
            data[tilde_off + 15],
        ]);
        let mut q = tilde_off + 24;
        let present: Vec<u8> = (0..64).filter(|i| valid & (1u64 << i) != 0).collect();
        let rows: Vec<u32> = present
            .iter()
            .map(|_| {
                let r = u32_at(&data, q);
                q += 4;
                r
            })
            .collect();

        let str_wide = heap_sizes & 1 != 0;
        let guid_wide = heap_sizes & 2 != 0;
        let blob_wide = heap_sizes & 4 != 0;
        let sidx = if str_wide { 4 } else { 2 };
        let gidx = if guid_wide { 4 } else { 2 };
        let bidx = if blob_wide { 4 } else { 2 };

        let row_of = |tag: u8| -> Option<u32> {
            present
                .iter()
                .position(|&t| t == tag)
                .map(|i| rows[i])
        };

        let simple = |tag: u8| -> u32 { row_of(tag).unwrap_or(0) };
        let max_rows = |tags: &[u8]| -> u32 {
            tags.iter().map(|&t| simple(t)).max().unwrap_or(0)
        };
        let coded = |tags: &[u8]| -> usize {
            if max_rows(tags) > 0xFFFF {
                4
            } else {
                2
            }
        };
        let module_row = simple(0x00);
        let _ = module_row;
        let typeref_rows = simple(0x01);
        let _ = typeref_rows;
        let typedef_rows_n = simple(0x02);
        let field_rows_n = simple(0x04);
        let method_rows_n = simple(0x06);

        let resolution_scope_w = coded(&[0x00, 0x1A, 0x23, 0x01]);
        let tdor_w = coded(&[0x02, 0x01, 0x1B]);
        let field_w = if field_rows_n > 0xFFFF { 4 } else { 2 };
        let method_w = if method_rows_n > 0xFFFF { 4 } else { 2 };
        let param_w = if simple(0x08) > 0xFFFF { 4 } else { 2 };

        let mut row_sizes: [usize; 0x0B] = [0; 0x0B];
        for i in 0..0x0B {
            let tag = i as u8;
            if row_of(tag).is_none() {
                continue;
            }
            row_sizes[tag as usize] = match tag {
                0x00 => 2 + sidx + gidx * 3,
                0x01 => resolution_scope_w + sidx * 2,
                0x02 => 4 + sidx * 2 + tdor_w + field_w + method_w,
                0x03 => field_w,
                0x04 => 2 + sidx + bidx,
                0x05 => method_w,
                0x06 => 4 + 2 + 2 + sidx + bidx + param_w,
                0x07 => param_w,
                0x08 => 2 + 2 + sidx,
                0x09 => {
                    let w = if typedef_rows_n > 0xFFFF { 4 } else { 2 };
                    let i_w = coded(&[0x02, 0x01, 0x1B]);
                    w + i_w
                }
                0x0A => {
                    let mrp_w = coded(&[0x02, 0x01, 0x1A, 0x06, 0x1B]);
                    mrp_w + sidx + bidx
                }
                _ => 0,
            };
        }
        let mut base = q;
        let mut table_off: [usize; 0x0B] = [0; 0x0B];
        for i in 0..0x0B {
            let tag = i as u8;
            if row_of(tag).is_some() {
                table_off[tag as usize] = base;
                base += row_sizes[tag as usize] * simple(tag) as usize;
            }
        }

        let read_s = |idx: u32| -> String {
            let p = strings_base + idx as usize;
            let mut e = p;
            while e < data.len() && data[e] != 0 {
                e += 1;
            }
            String::from_utf8_lossy(&data[p..e]).to_string()
        };

        let mut typedefs = Vec::new();
        {
            let t_off = table_off[0x02];
            let rs = row_sizes[0x02];
            for i in 0..typedef_rows_n as usize {
                let r = t_off + i * rs;
                let mut c = r + 4;
                let name_i = if sidx == 4 {
                    let v = u32_at(&data, c);
                    c += 4;
                    v
                } else {
                    let v = u16_at(&data, c);
                    c += 2;
                    v as u32
                };
                let ns_i = if sidx == 4 {
                    let v = u32_at(&data, c);
                    c += 4;
                    v
                } else {
                    let v = u16_at(&data, c);
                    c += 2;
                    v as u32
                };
                c += tdor_w as usize;
                let f_start = if field_w == 4 {
                    let v = u32_at(&data, c);
                    c += 4;
                    v
                } else {
                    let v = u16_at(&data, c);
                    c += 2;
                    v as u32
                };
                let m_start = if method_w == 4 {
                    u32_at(&data, c)
                } else {
                    u16_at(&data, c) as u32
                };
                let m_end = if i + 1 < typedef_rows_n as usize {
                    let nr = t_off + (i + 1) * rs;
                    let cc = nr + 4 + sidx as usize * 2 + tdor_w as usize + field_w as usize;
                    if method_w == 4 {
                        u32_at(&data, cc)
                    } else {
                        u16_at(&data, cc) as u32
                    }
                } else {
                    method_rows_n + 1
                };
                let f_end = if i + 1 < typedef_rows_n as usize {
                    let nr = t_off + (i + 1) * rs;
                    let cc = nr + 4 + sidx as usize * 2 + tdor_w as usize;
                    if field_w == 4 {
                        u32_at(&data, cc)
                    } else {
                        u16_at(&data, cc) as u32
                    }
                } else {
                    field_rows_n + 1
                };
                typedefs.push(TypeDefRow {
                    name: read_s(name_i),
                    namespace: read_s(ns_i),
                    method_start: m_start,
                    method_end: m_end,
                    field_start: f_start,
                    field_end: f_end,
                });
            }
        }

        let mut methods = Vec::new();
        {
            let t_off = table_off[0x06];
            let rs = row_sizes[0x06];
            for i in 0..method_rows_n as usize {
                let r = t_off + i * rs;
                let rva = u32_at(&data, r);
                let name_i = if sidx == 4 {
                    u32_at(&data, r + 8)
                } else {
                    u16_at(&data, r + 8) as u32
                };
                methods.push(MethodRow {
                    rva,
                    name: read_s(name_i),
                });
            }
        }

        let mut fields = Vec::new();
        {
            let t_off = table_off[0x04];
            let rs = row_sizes[0x04];
            for i in 0..field_rows_n as usize {
                let r = t_off + i * rs;
                let name_i = if sidx == 4 {
                    u32_at(&data, r + 2)
                } else {
                    u16_at(&data, r + 2) as u32
                };
                fields.push(FieldRow { name: read_s(name_i) });
            }
        }

        let mut typerefs = Vec::new();
        {
            let tag = 0x01;
            if row_of(tag).is_some() && (tag as usize) < table_off.len() {
                let t_off = table_off[tag as usize];
                let rs = row_sizes[tag as usize];
                let rs_w = resolution_scope_w;
                for i in 0..typeref_rows as usize {
                    let r = t_off + i * rs;
                    let name_i = if sidx == 4 {
                        u32_at(&data, r + rs_w as usize)
                    } else {
                        u16_at(&data, r + rs_w as usize) as u32
                    };
                    typerefs.push(read_s(name_i));
                }
            }
        }
        let mut memberrefs = Vec::new();
        {
            let tag = 0x0A;
            let has_tag = row_of(tag).is_some();
            let t_off = if has_tag { table_off.get(tag as usize).copied() } else { None };
            let r_size = if has_tag { row_sizes.get(tag as usize).copied() } else { None };
            if let (Some(t_off), Some(rs)) = (t_off, r_size) {
                let mrp_w = {
                    let max_rows = [
                        simple(0x02),
                        simple(0x01),
                        simple(0x1A),
                        simple(0x06),
                        simple(0x1B),
                    ]
                    .iter()
                    .max()
                    .copied()
                    .unwrap_or(0);
                    if max_rows > 0xFFFF {
                        4
                    } else {
                        2
                    }
                };
                for i in 0..simple(tag) as usize {
                    let r = t_off + i * rs;
                    let coded = if mrp_w == 4 {
                        u32_at(&data, r)
                    } else {
                        u16_at(&data, r) as u32
                    };
                    let (ctag, crid) = ((coded & 0x7) as u8, coded >> 3);
                    let name_i = if sidx == 4 {
                        u32_at(&data, r + mrp_w as usize)
                    } else {
                        u16_at(&data, r + mrp_w as usize) as u32
                    };
                    memberrefs.push((ctag, crid, read_s(name_i)));
                }
            }
        }

        Ok(CliImage {
            data,
            strings_base,
            us_base,
            us_size,
            typedefs,
            methods,
            fields,
            typerefs,
            memberrefs,
        })
    }

    pub fn find_us_token<F: Fn(&str) -> bool>(&self, pred: F) -> Option<u32> {
        let d = &self.data;
        let base = self.us_base;
        let end = (base + self.us_size).min(d.len());
        let mut i = base + 1;
        while i < end {
            let rec = i;
            let b0 = d[i];
            let (len, hdr) = if b0 & 0x80 == 0 {
                (b0 as usize, 1)
            } else if b0 & 0xC0 == 0x80 {
                (((b0 & 0x3F) as usize) << 8 | d[i + 1] as usize, 2)
            } else {
                (
                    ((b0 & 0x1F) as usize) << 24
                        | (d[i + 1] as usize) << 16
                        | (d[i + 2] as usize) << 8
                        | d[i + 3] as usize,
                    4,
                )
            };
            i += hdr;
            if len == 0 || i + len > end {
                break;
            }
            let blob = &d[i..i + len];
            i += len;
            let bytes: &[u8] = if len >= 2 { &blob[..len - 1] } else { &[] };
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let s = String::from_utf16_lossy(&units);
            if pred(&s) {
                return Some(0x7000_0000 | ((rec - base) as u32));
            }
        }
        None
    }

    pub fn find_type(&self, namespace: &str, name: &str) -> Option<(usize, &TypeDefRow)> {
        self.typedefs
            .iter()
            .position(|t| t.name == name && t.namespace == namespace)
            .map(|i| (i, &self.typedefs[i]))
    }

    pub fn find_method(&self, namespace: &str, type_name: &str, method: &str) -> Option<(u32, u32)> {
        let (_, t) = self.find_type(namespace, type_name)?;
        let (s, e) = (t.method_start, t.method_end);
        for rid in s..e {
            if let Some(m) = self.methods.get((rid - 1) as usize) {
                if m.name == method {
                    return Some((rid, m.rva));
                }
            }
        }
        None
    }

    pub fn method_token(&self, rid: u32) -> u32 {
        0x0600_0000 | rid
    }

    pub fn find_memberref_token(&self, type_name: &str, member: &str) -> Option<u32> {
        for (i, (ctag, crid, name)) in self.memberrefs.iter().enumerate() {
            if name != member || *ctag != 1 {
                continue;
            }
            let tr = self.typerefs.get(*crid as usize - 1)?;
            if tr == type_name {
                return Some(0x0A00_0000 | (i as u32 + 1));
            }
        }
        None
    }

    pub fn find_field_token(&self, namespace: &str, type_name: &str, field: &str) -> Option<u32> {
        let (_, t) = self.find_type(namespace, type_name)?;
        for rid in t.field_start..t.field_end {
            let f = self.fields.get((rid - 1) as usize)?;
            if f.name == field {
                return Some(0x0400_0000 | rid);
            }
        }
        None
    }

    pub fn method_il(&self, rva: u32) -> Option<(usize, Vec<u8>)> {
        if rva == 0 {
            return None;
        }
        let d = &self.data;
        let off = self.rva_offset(rva)?;
        let b0 = d[off];
        if b0 & 3 == 2 {
            let size = (b0 >> 2) as usize;
            return Some((off + 1, d[off + 1..off + 1 + size].to_vec()));
        }
        if b0 & 3 != 3 {
            return None;
        }
        let hdr = ((d[off] as u16) | ((d[off + 1] as u16) << 8)) >> 12;
        let size = u32_at(d, off + 4) as usize;
        let code_off = off + hdr as usize * 4;
        Some((code_off, d[code_off..code_off + size].to_vec()))
    }

    fn rva_offset(&self, rva: u32) -> Option<usize> {
        let d = &self.data;
        let pe_off = u32_at(d, 0x3C) as usize;
        let coff = pe_off + 4;
        let num_sections = u16_at(d, coff + 2) as usize;
        let opt_size = u16_at(d, coff + 16) as usize;
        let sec_tab = coff + 20 + opt_size;
        for i in 0..num_sections {
            let s = sec_tab + i * 40;
            let vsize = u32_at(d, s + 8);
            let va = u32_at(d, s + 12);
            let raw = u32_at(d, s + 20);
            if rva >= va && rva < va + vsize.max(1) {
                return Some((rva - va + raw) as usize);
            }
        }
        None
    }
}
