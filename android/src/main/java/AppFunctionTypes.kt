package com.plugin.google_app_functions

import android.app.appsearch.GenericDocument
import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

/**
 * A type from the table `tauri-plugin-google-app-functions-build` generates: a base (`boolean`,
 * `int`, `long`, `float`, `double`, `string`, `unit` or `@` plus an object name), then `[]` for
 * a list, then `?` when nullable.
 */
internal class AppFunctionType(code: String) {
    val isNullable = code.endsWith("?")
    val isList = code.removeSuffix("?").endsWith("[]")
    val base = code.removeSuffix("?").removeSuffix("[]")
    val objectName: String? = if (base.startsWith("@")) base.substring(1) else null
}

internal class AppFunctionMember(val name: String, val type: AppFunctionType)

internal class AppFunctionSignature(
    /** Rust name the function is registered under. */
    val name: String,
    val params: List<AppFunctionMember>,
    val returns: AppFunctionType,
)

/**
 * Converts between the AppSearch documents the system passes and the JSON the Rust handlers
 * take, using the generated type table.
 *
 * Documents hold every property as an array: `long` for Int and Long, `double` for Float and
 * Double, nested documents for objects. The table tells a one-element list from a scalar and a
 * missing list from null.
 */
internal class AppFunctionTypes(
    val library: String,
    val functions: Map<String, AppFunctionSignature>,
    private val objects: Map<String, List<AppFunctionMember>>,
) {
    /** Converts [document] into a JSON object with one entry per member. */
    fun toJson(document: GenericDocument, members: List<AppFunctionMember>): JSONObject {
        val json = JSONObject()
        for (member in members) {
            val values = document.getProperty(member.name)
            val count = if (values == null) 0 else java.lang.reflect.Array.getLength(values)
            val value: Any = when {
                member.type.isList && values == null ->
                    if (member.type.isNullable) JSONObject.NULL else JSONArray()
                member.type.isList -> JSONArray().also { array ->
                    for (i in 0 until count) {
                        array.put(elementToJson(java.lang.reflect.Array.get(values, i), member.type))
                    }
                }
                count == 0 -> JSONObject.NULL
                else -> elementToJson(java.lang.reflect.Array.get(values, 0), member.type)
            }
            json.put(member.name, value)
        }
        return json
    }

    private fun elementToJson(value: Any?, type: AppFunctionType): Any = when (value) {
        is GenericDocument -> toJson(value, members(type))
        else -> value ?: JSONObject.NULL
    }

    /** Builds the response document holding [result] as the return value. */
    fun toResponse(result: Any?, type: AppFunctionType): GenericDocument {
        val builder = GenericDocument.Builder<GenericDocument.Builder<*>>("", "", "")
        if (type.base != "unit") {
            setProperty(builder, RETURN_VALUE, result, type)
        }
        return builder.build()
    }

    private fun toDocument(json: JSONObject, name: String): GenericDocument {
        val builder = GenericDocument.Builder<GenericDocument.Builder<*>>("", "", name)
        for (member in objects[name].orEmpty()) {
            setProperty(builder, member.name, json.opt(member.name), member.type)
        }
        return builder.build()
    }

    private fun setProperty(
        builder: GenericDocument.Builder<*>,
        name: String,
        value: Any?,
        type: AppFunctionType,
    ) {
        if (value == null || value == JSONObject.NULL) return
        val items: List<Any?> =
            if (type.isList) (value as JSONArray).let { a -> List(a.length()) { a.get(it) } }
            else listOf(value)
        when (type.base) {
            "boolean" ->
                builder.setPropertyBoolean(name, *BooleanArray(items.size) { items[it] as Boolean })
            "int", "long" ->
                builder.setPropertyLong(name, *LongArray(items.size) { (items[it] as Number).toLong() })
            "float", "double" ->
                builder.setPropertyDouble(name, *DoubleArray(items.size) { (items[it] as Number).toDouble() })
            "string" ->
                builder.setPropertyString(name, *Array(items.size) { items[it] as String })
            else -> {
                val objectName = checkNotNull(type.objectName) { "unknown type ${type.base}" }
                builder.setPropertyDocument(
                    name,
                    *Array(items.size) { toDocument(items[it] as JSONObject, objectName) },
                )
            }
        }
    }

    private fun members(type: AppFunctionType): List<AppFunctionMember> =
        objects[type.objectName].orEmpty()

    companion object {
        /** Asset written by `tauri-plugin-google-app-functions-build`. */
        private const val ASSET = "generated/tauri_app_functions.json"
        /** `ExecuteAppFunctionResponse.PROPERTY_RETURN_VALUE` */
        private const val RETURN_VALUE = "androidAppfunctionsReturnValue"

        @Volatile
        private var loaded: AppFunctionTypes? = null

        fun load(context: Context): AppFunctionTypes =
            loaded ?: synchronized(this) {
                loaded ?: parse(context.assets.open(ASSET).bufferedReader().use { it.readText() })
                    .also { loaded = it }
            }

        private fun parse(text: String): AppFunctionTypes {
            val json = JSONObject(text)
            val functions = json.getJSONObject("functions").let { all ->
                all.keys().asSequence().associateWith { id ->
                    val function = all.getJSONObject(id)
                    AppFunctionSignature(
                        name = function.getString("name"),
                        params = members(function.getJSONArray("params")),
                        returns = AppFunctionType(function.getString("returns")),
                    )
                }
            }
            val objects = json.getJSONObject("objects").let { all ->
                all.keys().asSequence().associateWith { name -> members(all.getJSONArray(name)) }
            }
            return AppFunctionTypes(json.getString("library"), functions, objects)
        }

        private fun members(array: JSONArray): List<AppFunctionMember> = List(array.length()) {
            val member = array.getJSONArray(it)
            AppFunctionMember(member.getString(0), AppFunctionType(member.getString(1)))
        }
    }
}
