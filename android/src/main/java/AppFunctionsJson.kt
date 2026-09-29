package com.plugin.google_app_functions

import org.json.JSONArray
import org.json.JSONObject

/** JSON conversions used by the generated `TauriAppFunctionService`. */
object AppFunctionsJson {
    fun isNull(value: Any?): Boolean = value == null || value == JSONObject.NULL

    // Decoding: JSON value -> Kotlin

    fun boolean(value: Any?): Boolean = value as Boolean
    fun int(value: Any?): Int = (value as Number).toInt()
    fun long(value: Any?): Long = (value as Number).toLong()
    fun float(value: Any?): Float = (value as Number).toFloat()
    fun double(value: Any?): Double = (value as Number).toDouble()
    fun string(value: Any?): String = value as String
    fun obj(value: Any?): JSONObject = value as JSONObject

    fun <T> nullable(value: Any?, decode: (Any?) -> T): T? =
        if (isNull(value)) null else decode(value)

    fun <T> list(value: Any?, decode: (Any?) -> T): List<T> {
        val array = value as JSONArray
        return List(array.length()) { decode(array.opt(it)) }
    }

    fun booleanArray(value: Any?): BooleanArray =
        (value as JSONArray).let { a -> BooleanArray(a.length()) { a.getBoolean(it) } }
    fun intArray(value: Any?): IntArray =
        (value as JSONArray).let { a -> IntArray(a.length()) { a.getInt(it) } }
    fun longArray(value: Any?): LongArray =
        (value as JSONArray).let { a -> LongArray(a.length()) { a.getLong(it) } }
    fun floatArray(value: Any?): FloatArray =
        (value as JSONArray).let { a -> FloatArray(a.length()) { a.getDouble(it).toFloat() } }
    fun doubleArray(value: Any?): DoubleArray =
        (value as JSONArray).let { a -> DoubleArray(a.length()) { a.getDouble(it) } }

    // Encoding: Kotlin -> value accepted by JSONObject.put

    fun <T : Any> encodeNullable(value: T?, encode: (T) -> Any): Any =
        if (value == null) JSONObject.NULL else encode(value)

    fun <T> encodeList(values: List<T>, encode: (T) -> Any): JSONArray =
        JSONArray().also { array -> values.forEach { array.put(encode(it)) } }

    fun encodeArray(values: BooleanArray): JSONArray =
        JSONArray().also { array -> values.forEach { array.put(it) } }
    fun encodeArray(values: IntArray): JSONArray =
        JSONArray().also { array -> values.forEach { array.put(it) } }
    fun encodeArray(values: LongArray): JSONArray =
        JSONArray().also { array -> values.forEach { array.put(it) } }
    fun encodeArray(values: FloatArray): JSONArray =
        JSONArray().also { array -> values.forEach { array.put(it.toDouble()) } }
    fun encodeArray(values: DoubleArray): JSONArray =
        JSONArray().also { array -> values.forEach { array.put(it) } }
}
