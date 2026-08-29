import { useCallback, useEffect, useRef, useState } from 'react'
import { bridge } from '../lib/bridge'
import { chunkStorageFile, canTransitionStorage, type StorageState, type StorageEntry } from '../lib/storage'

type ConflictInfo = {
  path: string
  localMtime: number
  remoteMtime: number
  resolution: string
  winner: string
  loserRename: string
}

export function Storage() {
  const [path, setPath] = useState("/")
  const [entries, setEntries] = useState<StorageEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [syncState, setSyncState] = useState<StorageState>("IDLE")
  const [progress, setProgress] = useState<{id:string, name:string, pct:number}[]>([])
  const [conflict, setConflict] = useState<ConflictInfo | null>(null)
  const [newFolder, setNewFolder] = useState("")
  const [showPermDelete, setShowPermDelete] = useState<string | null>(null)
  const syncStateRef = useRef<StorageState>("IDLE")
  useEffect(()=>{ syncStateRef.current = syncState },[syncState])

  const doTransition = useCallback((to: StorageState) => {
    if (canTransitionStorage(syncStateRef.current, to)) {
      syncStateRef.current = to
      setSyncState(to)
    }
  },[])

  const ls = useCallback((p: string) => {
    setLoading(true); setError(null)
    if (syncStateRef.current === "IDLE") doTransition("SCANNING")
    bridge.send("storage.ls", { path: p, showHidden: false })
  }, [doTransition])

  const statAndLs = useCallback(() => ls(path), [ls, path])

  // One-time setup for WS handlers
  useEffect(() => {
    const onLs = (m: any) => {
      if (m.type === "storage.ls") {
        const payload = m.payload
        if (payload.entries) {
          // Only update if payload.path matches our current path? But we accept any for now.
          // To avoid stale, we still update.
          setEntries(payload.entries)
          setLoading(false)
          if (payload.truncated) setError("Truncated: >5000 entries")
          if (syncStateRef.current === "SCANNING") {
            doTransition("DONE")
            setTimeout(()=> doTransition("IDLE"), 1000)
          }
        } else if (payload.error) {
          setError(payload.error)
          setLoading(false)
        }
      }
    }
    const onStat = (_m: any) => {}
    const onMkdir = (m: any) => {
      if (m.type === "storage.mkdir") {
        if (m.payload.ok) {
          const parent = m.payload.path.substring(0, m.payload.path.lastIndexOf("/")) || "/"
          ls(parent === "" ? "/" : parent)
        } else setError(m.payload.error || "mkdir failed")
      }
    }
    const onRm = (m: any) => {
      if (m.type === "storage.rm") {
        if (m.payload.ok) ls(path)
        else setError(m.payload.error || "rm failed")
      }
    }
    const onSync = (m: any) => {
      if (m.type === "storage.sync") {
        const p = m.payload
        if (p.conflict) {
          doTransition("CONFLICT")
          setConflict({
            path: p.path,
            localMtime: 0,
            remoteMtime: 0,
            resolution: p.resolution || "lww",
            winner: p.winner || "local",
            loserRename: p.loserRename || ""
          })
        } else if (p.received) {
          const id = p.id
          setProgress(prev=> {
            const exists = prev.find(x=>x.id===id)
            if (!exists) return prev
            return prev.map(x=> x.id===id ? {...x, pct:100}:x)
          })
          doTransition("DONE")
          setTimeout(()=> doTransition("IDLE"), 800)
          setTimeout(()=> ls(path), 300)
        }
        if (p.error) setError(p.error)
      }
    }
    const onConflict = (m: any) => {
      if (m.type === "storage.conflict") {
        doTransition("CONFLICT")
        setConflict({
          path: m.payload.path,
          localMtime: m.payload.localMtime || 0,
          remoteMtime: m.payload.remoteMtime || 0,
          resolution: m.payload.resolution || "lww",
          winner: m.payload.winner || "local",
          loserRename: m.payload.loserRename || ""
        })
      }
    }
    const onError = (m:any) => {
      if (m.type==="error") {
        const code = m.payload.code
        if (["path_traversal","validation","missing_permission","saf_revoked","sha_mismatch"].includes(code)) {
          setError(`${code}: ${m.payload.message||m.payload.error}`)
          setLoading(false)
        }
      }
    }
    bridge.on("storage.ls", onLs)
    bridge.on("storage.stat", onStat)
    bridge.on("storage.mkdir", onMkdir)
    bridge.on("storage.rm", onRm)
    bridge.on("storage.sync", onSync)
    bridge.on("storage.conflict", onConflict)
    bridge.on("error", onError)
    return ()=>{
      bridge.off("storage.ls", onLs)
      bridge.off("storage.stat", onStat)
      bridge.off("storage.mkdir", onMkdir)
      bridge.off("storage.rm", onRm)
      bridge.off("storage.sync", onSync)
      bridge.off("storage.conflict", onConflict)
      bridge.off("error", onError)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Initial ls only once
  useEffect(()=>{ ls("/") }, [ls])

  const goUp = () => {
    if (path === "/") return
    const parts = path.split("/").filter(Boolean)
    parts.pop()
    const up = "/" + parts.join("/")
    const np = up === "/" ? "/" : up
    const finalPath = np || "/"
    setPath(finalPath)
    ls(finalPath)
  }

  const enterDir = (entry: StorageEntry) => {
    if (!entry.isDir) return
    setPath(entry.path)
    ls(entry.path)
  }

  const doMkdir = () => {
    if (!newFolder.trim()) return
    const target = path === "/" ? `/${newFolder.trim()}` : `${path}/${newFolder.trim()}`
    bridge.send("storage.mkdir", { path: target })
    setNewFolder("")
  }

  const doRm = (entry: StorageEntry, toTrash = true) => {
    if (!toTrash && showPermDelete !== entry.path) {
      setShowPermDelete(entry.path)
      return
    }
    setShowPermDelete(null)
    bridge.send("storage.rm", { path: entry.path, toTrash })
  }

  const doSyncFiles = async (files: FileList | null) => {
    if (!files) return
    for (const file of Array.from(files)) {
      const id = crypto.randomUUID()
      const relPath = path === "/" ? `/${file.name}` : `${path}/${file.name}`
      bridge.send("storage.stat", { path: relPath })
      doTransition("SYNCING")
      const chunks = await chunkStorageFile(id, relPath, await file.arrayBuffer())
      if (chunks.length===0) continue
      setProgress(prev=> [...prev, {id, name: file.name, pct: 0}])
      for (let i=0;i<chunks.length;i++) {
        const c = chunks[i]
        bridge.send("storage.sync", {
          id: c.id,
          path: c.path,
          size: c.size,
          offset: c.offset,
          total: c.total,
          index: c.index,
          sha256: c.sha256,
          data_b64: c.data_b64,
          mtimeMs: Date.now(),
          vectorClock: { desktop: 1 }
        })
        const pct = Math.round(((i+1)/chunks.length)*100)
        setProgress(prev=> prev.map(p=> p.id===id ? {...p, pct}:p))
        if (i % 20 === 19) await new Promise(r=> setTimeout(r, 100))
      }
    }
  }

  const [drag, setDrag] = useState(false)
  const onDrop = useCallback(async (e: React.DragEvent)=>{
    e.preventDefault(); setDrag(false)
    const files = e.dataTransfer.files
    await doSyncFiles(files)
  },[path])

  const resolveConflict = (choice: "local"|"remote"|"both") => {
    if (!conflict) return
    const winner = choice === "both" ? "local" : choice
    const resolution = choice === "both" ? "rename" : "manual"
    bridge.send("storage.conflict", { path: conflict.path, resolution, winner, loserRename: conflict.loserRename })
    setConflict(null)
    doTransition("SYNCING")
    setTimeout(()=> doTransition("DONE"), 500)
    setTimeout(()=> doTransition("IDLE"), 1000)
  }

  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-white font-semibold">Storage Deep — Folder Sync</h3>
        <span className={`text-xs px-2 py-1 rounded-full border ${syncState==="IDLE"?"border-bridge-border text-bridge-muted": syncState==="SCANNING"?"border-amber-500 text-amber-400": syncState==="SYNCING"?"border-bridge-accent text-bridge-accent": syncState==="CONFLICT"?"border-red-500 text-red-400": "border-emerald-500 text-emerald-400"}`}>{syncState}</span>
      </div>
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <button onClick={goUp} className="px-3 py-1 rounded-full border border-bridge-border text-bridge-muted hover:text-white">↑ Up</button>
        <span className="text-white font-mono text-xs bg-[#0f0f14] border border-bridge-border rounded-full px-3 py-1">{path}</span>
        <button onClick={statAndLs} className="px-3 py-1 rounded-full border border-bridge-border text-bridge-muted hover:text-white">↻ Refresh</button>
        <span className="text-xs text-bridge-muted">{entries.length} items {loading?"· loading…":""}</span>
      </div>
      {error && <div className="text-xs text-amber-400 bg-amber-950/30 border border-amber-800 rounded-xl p-2">{error} <button onClick={()=> setError(null)} className="ml-2 text-white underline">dismiss</button></div>}
      {conflict && (
        <div className="bg-red-950/30 border border-red-800 rounded-xl p-3 space-y-2">
          <div className="text-sm text-red-300 font-semibold">Conflict on {conflict.path}</div>
          <div className="text-xs text-red-200">LWW picked <b>{conflict.winner}</b> (resolution: {conflict.resolution}); loser saved as <code className="text-white">{conflict.loserRename}</code></div>
          <div className="flex gap-2">
            <button onClick={()=> resolveConflict("local")} className="px-3 py-1 rounded-full bg-white text-black text-xs">Keep local</button>
            <button onClick={()=> resolveConflict("remote")} className="px-3 py-1 rounded-full border border-red-700 text-red-300 text-xs">Keep remote</button>
            <button onClick={()=> resolveConflict("both")} className="px-3 py-1 rounded-full border border-bridge-border text-white text-xs">Keep both (rename)</button>
          </div>
        </div>
      )}
      <div className="flex gap-2">
        <input value={newFolder} onChange={e=> setNewFolder(e.target.value)} placeholder="New folder name" className="flex-1 bg-[#0f0f14] border border-bridge-border rounded-full px-4 py-1.5 text-sm text-white placeholder:text-bridge-muted" />
        <button onClick={doMkdir} className="px-4 py-1.5 rounded-full bg-bridge-accent text-black text-sm font-semibold">mkdir</button>
        <label className="px-4 py-1.5 rounded-full border border-bridge-border text-bridge-muted text-sm cursor-pointer">
          Upload
          <input type="file" multiple className="hidden" onChange={e=> doSyncFiles(e.target.files)} />
        </label>
      </div>
      <div onDragOver={e=>{e.preventDefault(); setDrag(true)}} onDragLeave={()=> setDrag(false)} onDrop={onDrop}
        className={`border-2 border-dashed rounded-2xl p-4 text-center text-sm ${drag?'border-bridge-accent bg-bridge-accent/10':'border-bridge-border bg-[#0f0f14]'}`}>
        <span className="text-white">Drag & drop files/folders here</span> <span className="text-bridge-muted">— 1 MB chunks, SHA256, 4GB+ resume via offset</span>
      </div>
      <div className="border border-bridge-border rounded-2xl overflow-hidden">
        <div className="grid grid-cols-12 gap-2 px-4 py-2 bg-[#0f0f14] text-xs text-bridge-muted border-b border-bridge-border">
          <span className="col-span-6">Name</span><span className="col-span-2">Size</span><span className="col-span-2">Modified</span><span className="col-span-2">Actions</span>
        </div>
        {entries.length===0 && !loading && <div className="px-4 py-6 text-center text-sm text-bridge-muted">Empty folder — {path}</div>}
        {entries.map(e=> (
          <div key={e.path} className="grid grid-cols-12 gap-2 px-4 py-2 items-center hover:bg-white/[0.04] border-b border-bridge-border/50 last:border-0">
            <span className="col-span-6 flex items-center gap-2 min-w-0">
              <span className={`w-6 h-6 rounded-lg flex items-center justify-center text-xs ${e.isDir?'bg-amber-500/20 text-amber-400':'bg-bridge-accent/20 text-bridge-accent'}`}>{e.isDir?'📁':'📄'}</span>
              <button onClick={()=> e.isDir && enterDir(e)} className={`truncate text-left text-sm ${e.isDir?'text-white hover:underline':'text-bridge-muted'}`}>{e.name}</button>
            </span>
            <span className="col-span-2 text-xs text-bridge-muted">{e.isDir?'-': formatSize(e.size)}</span>
            <span className="col-span-2 text-xs text-bridge-muted">{formatMtime(e.mtimeMs)}</span>
            <span className="col-span-2 flex gap-1">
              <button onClick={()=> doRm(e, true)} className="text-xs px-2 py-1 rounded-full border border-bridge-border text-amber-300 hover:bg-amber-900/20">Trash</button>
              <button onClick={()=> doRm(e, showPermDelete===e.path ? false : true)} className={`text-xs px-2 py-1 rounded-full border ${showPermDelete===e.path?'border-red-500 bg-red-900/30 text-red-300':'border-bridge-border text-bridge-muted hover:text-red-300'}`}>{showPermDelete===e.path?'Confirm permanent?':'Delete'}</button>
            </span>
          </div>
        ))}
      </div>
      {progress.length>0 && (
        <div className="space-y-2">
          <div className="text-xs text-bridge-muted">Sync progress (storage.sync chunked, SHA256, 4GB+ resume)</div>
          {progress.slice(-5).reverse().map(p=> (
            <div key={p.id} className="flex items-center justify-between bg-[#0f0f14] border border-bridge-border rounded-xl p-3">
              <span className="text-sm text-white truncate">{p.name}</span>
              <span className={`text-xs ${p.pct===100?'text-emerald-400':'text-bridge-accent'}`}>{p.pct}% {p.pct===100?'✓':''}</span>
            </div>
          ))}
        </div>
      )}
      <div className="text-xs text-bridge-muted">Trash: <code className="text-white">~/.local/share/Trash</code> (freedesktop) · Phone trash via <code className="text-white">MediaStore.createTrashRequest</code> (Android 30+) · Conflict LWW with vector clock · SAF DocumentFile tree walk · notify watch on <code className="text-white">~/Bridge</code></div>
    </div>
  )
}
function formatSize(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024*1024) return `${(n/1024).toFixed(1)} KB`
  if (n < 1024*1024*1024) return `${(n/1024/1024).toFixed(1)} MB`
  return `${(n/1024/1024/1024).toFixed(2)} GB`
}
function formatMtime(ms: number): string {
  if (!ms) return "-"
  const d = new Date(ms)
  return d.toLocaleString()
}
