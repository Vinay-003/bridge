package com.bridge.android.telephony

import android.app.role.RoleManager
import android.content.Context
import android.content.Intent
import android.os.Build
import android.provider.Telephony
import androidx.activity.result.ActivityResultLauncher
import androidx.core.content.ContextCompat

object RoleHelper {

    fun isDialerRoleHeld(context: Context): Boolean {
        return try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                val rm = context.getSystemService(RoleManager::class.java) ?: return false
                rm.isRoleHeld(RoleManager.ROLE_DIALER)
            } else {
                // Fallback: check if we are default dialer via Telecom
                val telecom = context.getSystemService(android.telecom.TelecomManager::class.java)
                telecom?.defaultDialerPackage == context.packageName
            }
        } catch (_: Exception) { false }
    }

    fun isSmsRoleHeld(context: Context): Boolean {
        return try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                val rm = context.getSystemService(RoleManager::class.java) ?: return false
                rm.isRoleHeld(RoleManager.ROLE_SMS)
            } else {
                Telephony.Sms.getDefaultSmsPackage(context) == context.packageName
            }
        } catch (_: Exception) { false }
    }

    fun requestDialerRole(context: Context, launcher: ActivityResultLauncher<Intent>) {
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                val rm = context.getSystemService(RoleManager::class.java) ?: return
                if (!rm.isRoleHeld(RoleManager.ROLE_DIALER) && rm.isRoleAvailable(RoleManager.ROLE_DIALER)) {
                    val intent = rm.createRequestRoleIntent(RoleManager.ROLE_DIALER)
                    launcher.launch(intent)
                }
            } else {
                val telecom = context.getSystemService(android.telecom.TelecomManager::class.java) ?: return
                val intent = Intent(android.telecom.TelecomManager.ACTION_CHANGE_DEFAULT_DIALER)
                intent.putExtra(android.telecom.TelecomManager.EXTRA_CHANGE_DEFAULT_DIALER_PACKAGE_NAME, context.packageName)
                launcher.launch(intent)
            }
        } catch (_: Exception) {}
    }

    fun requestSmsRole(context: Context, launcher: ActivityResultLauncher<Intent>) {
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                val rm = context.getSystemService(RoleManager::class.java) ?: return
                if (!rm.isRoleHeld(RoleManager.ROLE_SMS) && rm.isRoleAvailable(RoleManager.ROLE_SMS)) {
                    val intent = rm.createRequestRoleIntent(RoleManager.ROLE_SMS)
                    launcher.launch(intent)
                }
            } else {
                val intent = Intent(Telephony.Sms.Intents.ACTION_CHANGE_DEFAULT)
                intent.putExtra(Telephony.Sms.Intents.EXTRA_PACKAGE_NAME, context.packageName)
                // Use generic launch
                // Note: this requires handling via startActivity
            }
        } catch (_: Exception) {}
    }
}
