# -*- coding: utf-8 -*-
import sys, ctypes
from ctypes import c_char_p, c_uint32, Structure, POINTER
class MtailS(Structure):
    pass

print "here"
mt = ctypes.cdll.LoadLibrary('/Users/joel.self/Projects/joel/multitail-lib/target/debug/libmtaillib.dylib')
mt.multi_tail_new.restype = POINTER(MtailS)
mt.multi_tail_new.argtypes= [POINTER(ctypes.c_char_p), ctypes.c_size_t]

print "here2"
class FFITuple(ctypes.Structure):
		_fields_ = [("line", ctypes.c_wchar_p),
								("thread", ctypes.c_size_t)]
class FFITupleArray(ctypes.Structure):
    _fields_ = [("lines", POINTER(FFITuple)),
                ("len", ctypes.c_size_t)]

def void_array_to_tuple_list(array, _func, _args):
    tuple_array = ctypes.cast(array.lines, POINTER(FFITuple))
    return [tuple_array[i] for i in range(0, array.len)]

mt.get_received.restype = FFITupleArray
mt.get_received.argtypes = (POINTER(MtailS), )
mt.get_received.errcheck = void_array_to_tuple_list

class Mtail:
	def __init__(self, files, length):
		self.obj = mt.multi_tail_new(files, length)

	def __enter__(self):
		return self

	def __exit__(self, exc_type, exc_value, traceback):
		return

	def get_msgs(self):
		return mt.get_received(self.obj)


args = ['/Users/joel.self/Projects/joel/test3.log', '/Users/joel.self/Projects/joel/test4.log']

print "here3"
c_array = (ctypes.c_char_p * len(args))(*args)
with Mtail(c_array, len(args)) as tails:
	print "here4"
	while 1 == 1:
		print "here5"
		msgs = tails.get_msgs()
		print "here6"
		for i in range(0, len(msgs)):
			print "here7"
			print "%s" % msgs.lines[i].line

