package com.bridge.android.storage

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.work.*
import java.util.concurrent.TimeUnit

class SyncWorker(appContext: Context, params: WorkerParameters) : CoroutineWorker(appContext, params) {
    override suspend fun doWork(): Result {
        return try {
            if (Build.VERSION.SDK_INT >= 26) {
                try { setForeground(createForegroundInfo()) } catch (_: Exception) {}
            }
            val ctx = applicationContext
            val payload = org.json.JSONObject().apply { put("path", "/") }
            val ok = try {
                StorageHandler.handleLs(ctx, payload)
                true
            } catch (_: Exception) { false }
            if (ok) Result.success() else Result.retry()
        } catch (_: Exception) { Result.retry() }
    }

    private fun createForegroundInfo(): ForegroundInfo {
        val channelId = "bridge-sync"
        val nm = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                val ch = NotificationChannel(channelId, "Bridge Sync", NotificationManager.IMPORTANCE_LOW)
                nm.createNotificationChannel(ch)
            }
        } catch (_: Exception) {}
        val notif = NotificationCompat.Builder(applicationContext, channelId)
            .setContentTitle("Bridge Sync")
            .setContentText("Scanning storage...")
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setOngoing(true)
            .build()
        return ForegroundInfo(2, notif)
    }

    companion object {
        const val WORK_NAME = "bridge-storage-sync"
        fun enqueuePeriodic(ctx: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .setRequiresBatteryNotLow(false)
                .build()
            val req = PeriodicWorkRequestBuilder<SyncWorker>(15, TimeUnit.MINUTES)
                .setConstraints(constraints)
                .addTag(WORK_NAME)
                .build()
            WorkManager.getInstance(ctx).enqueueUniquePeriodicWork(WORK_NAME, ExistingPeriodicWorkPolicy.KEEP, req)
        }
        fun enqueueOneTime(ctx: Context) {
            val req = OneTimeWorkRequestBuilder<SyncWorker>().addTag(WORK_NAME).build()
            WorkManager.getInstance(ctx).enqueue(req)
        }
    }
}
