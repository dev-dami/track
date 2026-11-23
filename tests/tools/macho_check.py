#!/usr/bin/env python3
"""Independent structural checker for the Mach-O executables trackc writes
(docs/native.md, "Mach-O layout"). Written from Apple's <mach-o/loader.h>,
<mach-o/nlist.h> and the code-signing blob formats, not from macho.trk.

    python3 tests/tools/macho_check.py FILE...

Checks the header, the segments and sections (in the file, page-aligned,
not overlapping), LC_MAIN inside __text, the dyld bind opcodes (every
__got slot bound exactly once, to the symbol the indirect symbol table
names, from a loaded dylib), and the ad-hoc code signature: SuperBlob,
CodeDirectory fields, and every SHA-256 page hash recomputed. Prints
"ok FILE" per file; exits 1 with the first problem otherwise.
"""
import hashlib
import struct
import sys

LC_SEGMENT_64, LC_SYMTAB, LC_DYSYMTAB, LC_LOAD_DYLIB = 0x19, 0x2, 0xB, 0xC
LC_LOAD_DYLINKER, LC_UUID, LC_CODE_SIGNATURE, LC_BUILD_VERSION = 0xE, 0x1B, 0x1D, 0x32
LC_DYLD_INFO_ONLY, LC_MAIN = 0x80000022, 0x80000028
CPUS = {0x01000007: "x86_64", 0x0100000C: "arm64"}


class Bad(Exception):
    pass


def need(cond, msg):
    if not cond:
        raise Bad(msg)


def uleb(b, i):
    v = s = 0
    while True:
        x = b[i]
        i += 1
        v |= (x & 0x7F) << s
        s += 7
        if x < 0x80:
            return v, i


def cstr(b, i):
    j = b.index(b"\0", i)
    return b[i:j].decode(), j + 1


def check(path):
    f = open(path, "rb").read()
    magic, cpu, sub, ftype, ncmds, sizeofcmds, flags, _ = struct.unpack_from("<8I", f, 0)
    need(magic == 0xFEEDFACF, "not MH_MAGIC_64")
    need(cpu in CPUS, "cputype %#x" % cpu)
    need(ftype == 2, "not MH_EXECUTE")
    need(flags & 0x200085 == 0x200085, "flags %#x lack NOUNDEFS|DYLDLINK|TWOLEVEL|PIE" % flags)
    page = 0x4000
    segs, sects, dylibs, cmds = [], [], [], {}
    off = 32
    for _ in range(ncmds):
        cmd, size = struct.unpack_from("<2I", f, off)
        need(size % 8 == 0 and size >= 8, "load command size %d" % size)
        cmds.setdefault(cmd, []).append(off)
        if cmd == LC_SEGMENT_64:
            name = f[off + 8:off + 24].rstrip(b"\0").decode()
            va, vs, fo, fs, maxp, initp, ns, _ = struct.unpack_from("<4Q4I", f, off + 24)
            need(size == 72 + 80 * ns, "segment %s size" % name)
            segs.append((name, va, vs, fo, fs, initp))
            for k in range(ns):
                s = off + 72 + 80 * k
                sn = f[s:s + 16].rstrip(b"\0").decode()
                sg = f[s + 16:s + 32].rstrip(b"\0").decode()
                a, sz, so, al, _, _, fl, r1 = struct.unpack_from("<2Q6I", f, s + 32)
                need(sg == name, "section %s in %s" % (sn, name))
                need(va <= a and a + sz <= va + vs, "section %s outside its segment" % sn)
                need(so - fo == a - va, "section %s file/address mismatch" % sn)
                need(a % (1 << al) == 0, "section %s misaligned" % sn)
                sects.append((sn, sg, a, sz, so, fl, r1))
        elif cmd == LC_LOAD_DYLIB:
            dylibs.append(cstr(f, off + struct.unpack_from("<I", f, off + 8)[0])[0])
        off += size
    need(off == 32 + sizeofcmds, "sizeofcmds")
    for c in (LC_SYMTAB, LC_DYSYMTAB, LC_LOAD_DYLINKER, LC_UUID, LC_BUILD_VERSION,
              LC_DYLD_INFO_ONLY, LC_MAIN, LC_CODE_SIGNATURE):
        need(len(cmds.get(c, [])) == 1, "need exactly one load command %#x" % c)
    need(dylibs, "no LC_LOAD_DYLIB")
    need("/usr/lib/libSystem.B.dylib" in dylibs, "libSystem not loaded")
    dl = cmds[LC_LOAD_DYLINKER][0]
    need(cstr(f, dl + struct.unpack_from("<I", f, dl + 8)[0])[0] == "/usr/lib/dyld", "dylinker")
    bv = cmds[LC_BUILD_VERSION][0]
    plat, minos = struct.unpack_from("<2I", f, bv + 8)
    need(plat == 1 and minos >= 0xB0000, "build version platform %d minos %#x" % (plat, minos))
    # segments
    names = [s[0] for s in segs]
    need(names[0] == "__PAGEZERO" and segs[0][3:5] == (0, 0) and segs[0][5] == 0, "__PAGEZERO")
    need(names[-1] == "__LINKEDIT", "__LINKEDIT last")
    need("__TEXT" in names, "no __TEXT")
    text = segs[names.index("__TEXT")]
    need(text[3] == 0, "__TEXT must map the header")
    end = 0
    for name, va, vs, fo, fs, prot in segs[1:]:
        need(va % page == 0 and fo % page == 0 and vs % page == 0, "segment %s not page-aligned" % name)
        need(fs <= vs and fo + fs <= len(f), "segment %s past the file" % name)
        need(fo >= end, "segment %s overlaps" % name)
        need(va - text[1] == fo, "segment %s address != file offset + base" % name)
        need(not (prot & 2 and prot & 4), "segment %s writable and executable" % name)
        end = fo + fs
    le = segs[-1]
    need(le[3] + le[4] == len(f), "__LINKEDIT does not end the file")
    textsec = [s for s in sects if s[0] == "__text"]
    need(len(textsec) == 1, "one __text")
    entry, _ = struct.unpack_from("<2Q", f, cmds[LC_MAIN][0] + 8)
    need(textsec[0][4] <= entry < textsec[0][4] + textsec[0][3], "LC_MAIN entry outside __text")
    # symbols and binds
    so_, ns_, stro, strs = struct.unpack_from("<4I", f, cmds[LC_SYMTAB][0] + 8)
    syms = []
    for k in range(ns_):
        strx, ty, sc, desc, val = struct.unpack_from("<IBBHQ", f, so_ + 16 * k)
        need(strx < strs, "symbol name offset")
        syms.append((cstr(f, stro + strx)[0], ty, desc))
    ds = struct.unpack_from("<18I", f, cmds[LC_DYSYMTAB][0] + 8)
    need(ds[4] + ds[5] == ns_, "undefined symbols not last")
    indirect = struct.unpack_from("<%dI" % ds[13], f, ds[12]) if ds[13] else ()
    got = [s for s in sects if s[0] == "__got"]
    need(len(got) == 1 and got[0][5] & 0xFF == 6, "one __got of non-lazy pointers")
    g = got[0]
    nslot = g[3] // 8
    want = {}
    for k in range(nslot):
        name, ty, desc = syms[indirect[g[6] + k]]
        need(ty == 1, "__got slot %d symbol %s not undefined-external" % (k, name))
        need(1 <= desc >> 8 <= len(dylibs), "symbol %s library ordinal" % name)
        want[g[2] + 8 * k] = (name, desc >> 8)
    di = struct.unpack_from("<10I", f, cmds[LC_DYLD_INFO_ONLY][0] + 8)
    need(di[1] == 0 or di[0] >= le[3], "rebase info outside __LINKEDIT")
    b, i, stop = f, di[2], di[2] + di[3]
    ordn = sym = seg = segoff = None
    bound = {}
    while i < stop:
        op, imm = b[i] & 0xF0, b[i] & 0x0F
        i += 1
        if op == 0x00:
            break
        elif op == 0x10:
            ordn = imm
        elif op == 0x20:
            ordn, i = uleb(b, i)
        elif op == 0x40:
            sym, i = cstr(b, i)
        elif op == 0x50:
            need(imm == 1, "bind type %d" % imm)
        elif op == 0x70:
            seg = imm
            segoff, i = uleb(b, i)
        elif op == 0x90:
            need(seg is not None and sym is not None and ordn, "incomplete bind")
            addr = segs[seg][1] + segoff
            need(addr not in bound, "slot %#x bound twice" % addr)
            bound[addr] = (sym, ordn)
            segoff += 8
        else:
            raise Bad("unexpected bind opcode %#x" % b[i - 1])
    need(bound == want, "binds do not match the __got / indirect symbols")
    # code signature
    cs, cssize = struct.unpack_from("<2I", f, cmds[LC_CODE_SIGNATURE][0] + 8)
    need(cs % 16 == 0 and cs >= le[3] and cs + cssize == len(f), "signature placement")
    mg, ln, cnt = struct.unpack_from(">3I", f, cs)
    need(mg == 0xFADE0CC0 and ln <= cssize, "SuperBlob")
    cd = None
    for k in range(cnt):
        ty, o = struct.unpack_from(">2I", f, cs + 12 + 8 * k)
        if ty == 0:
            cd = cs + o
    need(cd is not None, "no CodeDirectory slot")
    (cmg, cln, ver, cflags, hoff, ioff, nspec, ncode, limit) = struct.unpack_from(">9I", f, cd)
    hsize, htype, _plat, pshift = struct.unpack_from(">4B", f, cd + 36)
    need(cmg == 0xFADE0C02, "CodeDirectory magic")
    need(cflags & 0x20002 == 0x20002, "flags %#x lack adhoc|linker-signed" % cflags)
    need(hsize == 32 and htype == 2 and pshift == 12, "not SHA-256 over 4 KiB pages")
    need(limit == cs, "codeLimit %d != signature offset %d" % (limit, cs))
    need(ncode == (limit + 4095) // 4096, "nCodeSlots")
    need(cd + cln <= cs + ln, "CodeDirectory past the SuperBlob")
    if ver >= 0x20400:
        base, lim, segflags = struct.unpack_from(">3Q", f, cd + 64)
        need(base == text[3] and lim == text[4] and segflags & 1, "exec segment != __TEXT")
    for p in range(ncode):
        page_bytes = f[4096 * p:min(4096 * (p + 1), limit)]
        h = f[cd + hoff + 32 * p:cd + hoff + 32 * (p + 1)]
        need(hashlib.sha256(page_bytes).digest() == h, "page %d hash mismatch" % p)
    return "%s %s, %d imports, %d pages hashed" % (CPUS[cpu], cstr(f, cd + ioff)[0], nslot, ncode)


def main():
    rc = 0
    for p in sys.argv[1:]:
        try:
            print("ok %s: %s" % (p, check(p)))
        except (Bad, struct.error, ValueError, IndexError) as e:
            print("FAIL %s: %s" % (p, e))
            rc = 1
    return rc


if __name__ == "__main__":
    sys.exit(main())
