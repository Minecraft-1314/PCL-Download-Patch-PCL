using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using dnlib.DotNet;
using dnlib.DotNet.Emit;
using dnlib.DotNet.Writer;

internal static class Program
{
	private class ErrLogger : ILogger
	{
		public bool IgnoresEvent(LoggerEvent e)
		{
			return e != LoggerEvent.Error;
		}

		public void Log(object sender, LoggerEvent e, string format, params object[] args)
		{
			string text = ((args.Length != 0) ? string.Format(format, args) : format);
			Console.WriteLine("METAERR: " + text);
			StackFrame[] frames = new StackTrace(2, fNeedFileInfo: true).GetFrames();
			if (frames == null)
			{
				return;
			}
			for (int i = 0; i < frames.Length && i < 5; i++)
			{
				MethodBase method = frames[i].GetMethod();
				if (method != null)
				{
					Console.WriteLine("  at " + method.DeclaringType?.Name + "." + method.Name);
				}
			}
		}
	}

	private static bool FullTransplant;

	private static int Fail(string msg)
	{
		Console.WriteLine("ERROR: " + msg);
		return 1;
	}

	private static ITypeDefOrRef MapTypeStatic(ITypeDefOrRef? tdr, ModuleDef mod)
	{
		if (tdr == null)
		{
			return tdr;
		}
		string fullName = tdr.FullName;
		if (fullName.Contains("GetResults"))
		{
			Console.WriteLine("DBGGR " + fullName);
		}
		string text = MapClosureTypeName(fullName);
		if (text != fullName)
		{
			TypeDef typeDef = mod.Find(text, isReflectionName: false);
			if (typeDef != null)
			{
				return typeDef;
			}
		}
		if (fullName.StartsWith("PCL."))
		{
			if (fullName == "PCL.ValidateBase")
			{
				TypeDef typeDef2 = mod.Find("PCL.Validate", isReflectionName: false);
				if (typeDef2 != null)
				{
					return typeDef2;
				}
			}
			TypeDef typeDef3 = mod.Find(fullName, isReflectionName: false);
			if (typeDef3 != null)
			{
				return typeDef3;
			}
		}
		string text2 = fullName.Replace("LoaderCombo_1`1", "LoaderCombo`1").Replace("LoaderTask_2`2", "LoaderTask`2").Replace("MyVirtualizingElement_1`1", "MyVirtualizingElement`1");
		int num = fullName.IndexOf("VB_AnonymousDelegate_");
		if (num >= 0)
		{
			string text3 = fullName.Substring(num + "VB_AnonymousDelegate_".Length);
			int num2 = text3.IndexOf('`');
			int num3 = text3.IndexOf('_');
			text2 = ((num3 <= 0 || (num2 >= 0 && num3 >= num2)) ? (fullName.Substring(0, num) + "VB$AnonymousDelegate_" + text3) : (fullName.Substring(0, num) + "VB$AnonymousDelegate_" + text3.Substring(0, num3) + ((num2 >= 0) ? text3.Substring(num2) : "")));
		}
		if (text2 != fullName)
		{
			TypeDef typeDef4 = mod.Find(text2, isReflectionName: false);
			if (typeDef4 != null)
			{
				return typeDef4;
			}
		}
		return tdr;
	}

	private static TypeSig RemapSigStatic(TypeSig? sig, ModuleDef mod)
	{
		if (sig == null)
		{
			return null;
		}
		if (sig is GenericInstSig genericInstSig)
		{
			ITypeDefOrRef typeDefOrRef = MapTypeStatic(genericInstSig.GenericType.TypeDefOrRef, mod);
			List<TypeSig> list = new List<TypeSig>();
			foreach (TypeSig genericArgument in genericInstSig.GenericArguments)
			{
				list.Add(RemapSigStatic(genericArgument, mod));
			}
			ClassOrValueTypeSig classOrValueTypeSig = ((typeDefOrRef is TypeDef { IsValueType: not false } || (typeDefOrRef is TypeRef && (typeDefOrRef.FullName.StartsWith("System.ValueTuple`") || typeDefOrRef.FullName == "System.Nullable`1" || typeDefOrRef.FullName.StartsWith("MeloongCore.OneOf`") || typeDefOrRef.FullName.EndsWith("/Enumerator") || typeDefOrRef.FullName == "/Enumerator" || typeDefOrRef.FullName.EndsWith("/KeyCollection") || typeDefOrRef.FullName.EndsWith("/ValueCollection") || typeDefOrRef.FullName.StartsWith("System.Collections.Generic.KeyValuePair`")))) ? ((ClassOrValueTypeSig)new ValueTypeSig(typeDefOrRef)) : ((ClassOrValueTypeSig)new ClassSig(typeDefOrRef)));
			if (list.Count == 0)
			{
				return classOrValueTypeSig;
			}
			GenericInstSig genericInstSig2 = new GenericInstSig(classOrValueTypeSig, list[0]);
			for (int i = 1; i < list.Count; i++)
			{
				genericInstSig2.GenericArguments.Add(list[i]);
			}
			return genericInstSig2;
		}
		if (sig is SZArraySig sZArraySig)
		{
			return new SZArraySig(RemapSigStatic(sZArraySig.Next, mod));
		}
		if (sig is ClassSig classSig)
		{
			return new ClassSig(MapTypeStatic(classSig.TypeDefOrRef, mod));
		}
		if (sig is ValueTypeSig valueTypeSig)
		{
			return new ValueTypeSig(MapTypeStatic(valueTypeSig.TypeDefOrRef, mod));
		}
		if (sig is ByRefSig byRefSig)
		{
			return new ByRefSig(RemapSigStatic(byRefSig.Next, mod));
		}
		if (sig is PtrSig ptrSig)
		{
			return new PtrSig(RemapSigStatic(ptrSig.Next, mod));
		}
		return sig;
	}

	private static GenericInstMethodSig RemapInstSig(GenericInstMethodSig inst, ModuleDef mod)
	{
		if (inst == null)
		{
			return null;
		}
		GenericInstMethodSig genericInstMethodSig = new GenericInstMethodSig();
		foreach (TypeSig genericArgument in inst.GenericArguments)
		{
			TypeSig item = RemapSigStatic(genericArgument, mod);
			genericInstMethodSig.GenericArguments.Add(item);
		}
		return genericInstMethodSig;
	}

	private static string MapClosureTypeName(string fullName)
	{
		if (fullName == null)
		{
			return fullName;
		}
		if (fullName.StartsWith("/") && !fullName.StartsWith("PCL.PageDownloadResourceDetail/"))
		{
			string text = fullName.Substring(1);
			if (text.StartsWith("_Closure___") || text == "<>c" || text.StartsWith("<>c__DisplayClass"))
			{
				fullName = "PCL.PageDownloadResourceDetail/" + text;
			}
		}
		if (!fullName.StartsWith("PCL.PageDownloadResourceDetail/"))
		{
			return fullName;
		}
		string text2 = fullName.Substring("PCL.PageDownloadResourceDetail/".Length);
		if (!FullTransplant)
		{
			if (text2 == "_Closure___")
			{
				return "PCL.PageDownloadResourceDetail/_Closure$__";
			}
			if (text2.StartsWith("_Closure___"))
			{
				string text3 = text2.Substring("_Closure___".Length);
				return "PCL.PageDownloadResourceDetail/_Closure$__" + text3.Replace("_", "-");
			}
			return fullName;
		}
		if (text2 == "_Closure___")
		{
			return "PCL.PageDownloadResourceDetail/_Closure$__R";
		}
		if (text2.StartsWith("_Closure___"))
		{
			string text4 = text2.Substring("_Closure___".Length).Replace("_", "-");
			return "PCL.PageDownloadResourceDetail/_Closure$__" + text4 + "R";
		}
		return fullName;
	}

	private static string MapClosureMethodName(string name)
	{
		if (name == null)
		{
			return name;
		}
		if (name.StartsWith("_Lambda___"))
		{
			string text = name.Substring("_Lambda___".Length);
			string text2 = text;
			int num = text.IndexOf('_');
			if (num > 0 && !text.Substring(0, num).StartsWith("R"))
			{
				text2 = text.Substring(0, num) + "-" + text.Substring(num + 1);
			}
			return "_Lambda$__" + text2;
		}
		return name;
	}

	private static MethodSig RemapMethodSigStatic(MethodSig? sig, ModuleDef mod)
	{
		if (sig == null)
		{
			return null;
		}
		List<TypeSig> list = new List<TypeSig>();
		foreach (TypeSig param in sig.Params)
		{
			TypeSig item = RemapSigStatic(param, mod);
			list.Add(item);
		}
		TypeSig retType = RemapSigStatic(sig.RetType, mod);
		MethodSig methodSig = ((!sig.HasThis) ? MethodSig.CreateStatic(retType, list.ToArray()) : MethodSig.CreateInstance(retType, list.ToArray()));
		if (sig.Generic)
		{
			methodSig.Generic = true;
			methodSig.GenParamCount = sig.GenParamCount;
		}
		return methodSig;
	}

	private static FieldDef? FindMatchingClosureField(string fieldName, string mappedDecl, ModuleDef mod)
	{
		if (mod == null)
		{
			return null;
		}
		string prefix = mappedDecl.Substring(0, mappedDecl.LastIndexOf('/') + 1);
		string bare = mappedDecl.Substring(mappedDecl.LastIndexOf('/') + 1);
		List<TypeDef> list = (from t in mod.GetTypes()
			where t.Name == bare && t.FullName.StartsWith(prefix)
			select t).ToList();
		foreach (TypeDef item in list)
		{
			FieldDef fieldDef = item.Fields.FirstOrDefault((FieldDef x) => x.Name == fieldName);
			if (fieldDef != null)
			{
				return fieldDef;
			}
		}
		foreach (TypeDef item2 in list)
		{
			FieldDef fieldDef2 = item2.Fields.FirstOrDefault((FieldDef x) => x.Name == fieldName || x.Name == fieldName.Replace("_VB_Local_", "$VB$Local_") || x.Name == fieldName.Replace("_VB_Me", "$VB$Me") || x.Name == fieldName.Replace("_VB_NonLocal__VB_Closure_", "$VB$NonLocal__VB_Closure_"));
			if (fieldDef2 != null)
			{
				return fieldDef2;
			}
		}
		return null;
	}

	private static void CopyBody(MethodDef srcMethod, MethodDef dstMethod, ModuleDef mod, Importer importer, MethodDef? miStr, Dictionary<string, IMethod> packs, IMethod? oneOfImplicitT1, IMethod? dstRunInThread, Dictionary<TypeDef, TypeDef> typeMap, TypeDef? srcPage, TypeDef? dstPage, bool isFwd, TypeDef? dstClosure, MethodDef? dstClosureCtor, MethodDef? dstClosureLambda, FieldDef? dstClosureFile, TypeDef? dstC22, TypeDef? dstC83, Dictionary<string, TypeDef>? cloneMap = null, FieldDef? dstTextField = null)
	{
		Dictionary<string, TypeDef> cloneMap2 = cloneMap;
		TypeDef dstClosure2 = dstClosure;
		MethodDef dstClosureCtor2 = dstClosureCtor;
		MethodDef dstClosureLambda2 = dstClosureLambda;
		TypeDef dstC84 = dstC22;
		MethodDef miStr2 = miStr;
		IMethod dstRunInThread2 = dstRunInThread;
		FieldDef dstTextField2 = dstTextField;
		CilBody cilBody2 = (dstMethod.Body = new CilBody());
		CilBody cilBody3 = cilBody2;
		cilBody3.MaxStack = 16;
		cilBody3.InitLocals = true;
		foreach (Local variable in srcMethod.Body.Variables)
		{
			TypeSig typeSig = RemapSigStatic(variable.Type, mod);
			if (cloneMap2 != null && variable.Type is ClassSig classSig && classSig.TypeDefOrRef?.FullName != null)
			{
				string fullName = classSig.TypeDefOrRef.FullName;
				if (cloneMap2.TryGetValue(fullName, out var value))
				{
					typeSig = new ClassSig(value);
				}
				else if (fullName.Contains("DisplayClass83_0") && dstC83 != null)
				{
					typeSig = new ClassSig(dstC83);
				}
				else if (fullName.Contains("DisplayClass22_0") && dstC84 != null)
				{
					typeSig = new ClassSig(dstC84);
				}
			}
			if (typeSig == null)
			{
				Console.WriteLine(string.Concat("  null local type in ", srcMethod.Name, " — skipped"));
				typeSig = new ClassSig(mod.CorLibTypes.Object.TypeDefOrRef);
			}
			cilBody3.Variables.Add(new Local(typeSig));
		}
		Dictionary<Instruction, Instruction> map = new Dictionary<Instruction, Instruction>();
		foreach (Instruction instruction4 in srcMethod.Body.Instructions)
		{
			Instruction instruction = new Instruction
			{
				OpCode = instruction4.OpCode
			};
			map[instruction4] = instruction;
			cilBody3.Instructions.Add(instruction);
		}
		for (int i = 0; i < srcMethod.Body.Instructions.Count; i++)
		{
			Instruction instruction2 = srcMethod.Body.Instructions[i];
			Instruction instruction3 = cilBody3.Instructions[i];
			switch (instruction2.OpCode.OperandType)
			{
			case OperandType.InlineBrTarget:
			case OperandType.ShortInlineBrTarget:
				instruction3.Operand = map[(Instruction)instruction2.Operand];
				break;
			case OperandType.InlineSwitch:
				instruction3.Operand = ((Instruction[])instruction2.Operand).Select((Instruction x) => map[x]).ToArray();
				break;
			case OperandType.InlineString:
				instruction3.Operand = (string)instruction2.Operand;
				break;
			case OperandType.InlineMethod:
			{
				IMethod operand2 = Resolve((IMethod)instruction2.Operand);
				if (instruction2.Operand is MethodDef methodDef && methodDef.DeclaringType == srcPage && methodDef.Name == "Install_Click" && isFwd)
				{
					operand2 = dstMethod;
				}
				instruction3.Operand = operand2;
				break;
			}
			case OperandType.InlineField:
			{
				if (instruction2.Operand is FieldDef fieldDef)
				{
					FieldDef fieldDef2 = ResolveField(fieldDef);
					IField field = fieldDef2;
					instruction3.Operand = field ?? importer.Import(fieldDef);
					break;
				}
				object operand = instruction2.Operand;
				MemberRef mf = operand as MemberRef;
				if (mf == null)
				{
					break;
				}
				string text = mf.DeclaringType?.FullName ?? "";
				IField field2 = null;
				TypeDef value3;
				if (text.Contains("DisplayClass13_0") && dstClosure2 != null)
				{
					field2 = dstTextField2 ?? dstClosure2.Fields.FirstOrDefault((FieldDef f2) => f2.Name == mf.Name);
				}
				else if (cloneMap2 != null && text.StartsWith("PCL.") && cloneMap2.TryGetValue(text, out value3))
				{
					field2 = value3.Fields.FirstOrDefault((FieldDef f2) => f2.Name == mf.Name);
				}
				if (field2 == null)
				{
					string text2 = MapClosureTypeName(text);
					Console.WriteLine(string.Concat("DBGFIELD ", mf.Name, " decl=", text, " mapped=", text2));
					if (text2 != text)
					{
						field2 = FindMatchingClosureField(mf.Name, text2, mod);
					}
					if (field2 == null)
					{
						Console.WriteLine("DBGFIELD-MISS " + mf.Name);
					}
				}
				instruction3.Operand = ((IMethod)field2) ?? Resolve(mf);
				break;
			}
			case OperandType.InlineSig:
			case OperandType.InlineTok:
			case OperandType.InlineType:
				if (instruction2.Operand is ITypeDefOrRef typeDefOrRef)
				{
					ITypeDefOrRef typeDefOrRef2;
					if (!(typeDefOrRef is TypeSpec typeSpec))
					{
						typeDefOrRef2 = MapTypeStatic(typeDefOrRef, mod);
					}
					else
					{
						ITypeDefOrRef typeDefOrRef3 = new TypeSpecUser(RemapSigStatic(typeSpec.TypeSig, mod));
						typeDefOrRef2 = typeDefOrRef3;
					}
					ITypeDefOrRef typeDefOrRef4 = typeDefOrRef2;
					if (cloneMap2 != null && typeDefOrRef4.FullName.StartsWith("PCL.") && cloneMap2.TryGetValue(typeDefOrRef4.FullName, out var value2))
					{
						instruction3.Operand = value2;
					}
					else if (typeDefOrRef4 != typeDefOrRef)
					{
						instruction3.Operand = typeDefOrRef4;
					}
					else if (typeDefOrRef.FullName.Contains("DisplayClass") && typeMap.Count > 0)
					{
						bool flag = false;
						foreach (KeyValuePair<TypeDef, TypeDef> item in typeMap)
						{
							if (typeDefOrRef.FullName == item.Key.FullName)
							{
								instruction3.Operand = item.Value;
								flag = true;
								break;
							}
						}
						if (!flag)
						{
							instruction3.Operand = importer.Import(typeDefOrRef);
						}
					}
					else
					{
						instruction3.Operand = importer.Import(typeDefOrRef);
					}
				}
				else if (instruction2.Operand is MemberRef src2)
				{
					instruction3.Operand = Resolve(src2);
				}
				break;
			default:
				instruction3.Operand = instruction2.Operand;
				break;
			case OperandType.InlineNone:
				break;
			}
		}
		foreach (ExceptionHandler exceptionHandler2 in srcMethod.Body.ExceptionHandlers)
		{
			ExceptionHandler exceptionHandler = new ExceptionHandler(exceptionHandler2.HandlerType);
			if (exceptionHandler2.TryStart != null && map.ContainsKey(exceptionHandler2.TryStart))
			{
				exceptionHandler.TryStart = map[exceptionHandler2.TryStart];
			}
			if (exceptionHandler2.TryEnd != null && map.ContainsKey(exceptionHandler2.TryEnd))
			{
				exceptionHandler.TryEnd = map[exceptionHandler2.TryEnd];
			}
			if (exceptionHandler2.HandlerStart != null && map.ContainsKey(exceptionHandler2.HandlerStart))
			{
				exceptionHandler.HandlerStart = map[exceptionHandler2.HandlerStart];
			}
			if (exceptionHandler2.HandlerEnd != null && map.ContainsKey(exceptionHandler2.HandlerEnd))
			{
				exceptionHandler.HandlerEnd = map[exceptionHandler2.HandlerEnd];
			}
			if (exceptionHandler2.FilterStart != null && map.ContainsKey(exceptionHandler2.FilterStart))
			{
				exceptionHandler.FilterStart = map[exceptionHandler2.FilterStart];
			}
			exceptionHandler.CatchType = ((exceptionHandler2.CatchType == null) ? null : importer.Import(exceptionHandler2.CatchType));
			cilBody3.ExceptionHandlers.Add(exceptionHandler);
		}
		foreach (ExceptionHandler exceptionHandler3 in cilBody3.ExceptionHandlers)
		{
			if (exceptionHandler3.HandlerEnd == null && cilBody3.Instructions.Count > 0)
			{
				exceptionHandler3.HandlerEnd = cilBody3.Instructions[cilBody3.Instructions.Count - 1];
			}
		}
		cilBody3.SimplifyBranches();
		cilBody3.OptimizeBranches();
		MethodSig? RemapMethodSig(MethodSig? sig)
		{
			if (sig == null)
			{
				return null;
			}
			MethodSig methodSig = new MethodSig
			{
				CallingConvention = sig.CallingConvention,
				HasThis = sig.HasThis,
				ExplicitThis = sig.ExplicitThis,
				Generic = sig.Generic,
				GenParamCount = sig.GenParamCount,
				RetType = RemapSigStatic(sig.RetType, mod)
			};
			foreach (TypeSig param in sig.Params)
			{
				methodSig.Params.Add(RemapSigStatic(param, mod));
			}
			return methodSig;
		}
		IMethod Resolve(IMethod src)
		{
			if (src is MethodSpec methodSpec)
			{
				if (methodSpec.Method is MethodDef methodDef2)
				{
					string fullName2 = methodDef2.DeclaringType?.FullName ?? "";
					TypeDef typeDef3 = mod.Find(fullName2, isReflectionName: false);
					if (typeDef3 != null)
					{
						foreach (MethodDef method in typeDef3.Methods)
						{
							if (method.Name == methodDef2.Name && method.Parameters.Count == methodDef2.Parameters.Count && method.IsStatic == methodDef2.IsStatic)
							{
								return new MethodSpecUser(method, RemapInstSig(methodSpec.GenericInstMethodSig, mod));
							}
						}
					}
					return importer.Import(methodSpec);
				}
				if (methodSpec.Method is MemberRef { DeclaringType: not null } memberRef)
				{
					ITypeDefOrRef typeDefOrRef5 = ((!(memberRef.DeclaringType is TypeSpec typeSpec2)) ? MapTypeStatic(memberRef.DeclaringType, mod) : new TypeSpecUser(RemapSigStatic(typeSpec2.TypeSig, mod)));
					if (cloneMap2 != null && typeDefOrRef5.FullName.StartsWith("PCL.") && cloneMap2.TryGetValue(typeDefOrRef5.FullName, out var value6))
					{
						typeDefOrRef5 = value6;
					}
					MethodSig sig2 = RemapMethodSigStatic(memberRef.MethodSig, mod);
					MemberRefUser memberRefUser = new MemberRefUser(mod, memberRef.Name, sig2, typeDefOrRef5);
					if (memberRefUser.MethodSig != null && memberRefUser.MethodSig.RetType != null)
					{
						memberRefUser.MethodSig.RetType = RemapSigStatic(memberRefUser.MethodSig.RetType, mod);
					}
					return new MethodSpecUser(memberRefUser, RemapInstSig(methodSpec.GenericInstMethodSig, mod));
				}
				return importer.Import(methodSpec);
			}
			if (src is MethodDef methodDef3)
			{
				string text8 = methodDef3.DeclaringType?.FullName ?? "";
				string text9 = MapClosureTypeName(text8);
				if (text9 != text8 && text9.StartsWith("PCL."))
				{
					TypeDef typeDef4 = mod.Find(text9, isReflectionName: false);
					if (typeDef4 != null)
					{
						foreach (MethodDef method2 in typeDef4.Methods)
						{
							if (method2.Name == MapClosureMethodName(methodDef3.Name) && method2.Parameters.Count == methodDef3.Parameters.Count && method2.IsStatic == methodDef3.IsStatic)
							{
								return method2;
							}
						}
					}
				}
				if (text8.Contains("DisplayClass13_0") && dstClosure2 != null)
				{
					if (methodDef3.Name == ".ctor")
					{
						return dstClosureCtor2;
					}
					if (methodDef3.Name.Contains("b__0"))
					{
						return dstClosureLambda2;
					}
				}
				if (cloneMap2 != null && text8.StartsWith("PCL.") && cloneMap2.TryGetValue(text8, out var value7))
				{
					foreach (MethodDef method3 in value7.Methods)
					{
						if (method3.Name == methodDef3.Name)
						{
							return method3;
						}
					}
				}
				if (text8.Contains("_Closure___22_0") && dstC84 != null)
				{
					MethodDef methodDef4 = null;
					foreach (MethodDef method4 in dstC84.Methods)
					{
						if (method4.Name == methodDef3.Name)
						{
							methodDef4 = method4;
							break;
						}
					}
					if (methodDef4 != null)
					{
						return methodDef4;
					}
				}
				if (text8 == "PCL.ModModpack")
				{
					if (methodDef3.Name == "ModpackInstall" && methodDef3.Parameters.Count > 1 && miStr2 != null)
					{
						return miStr2;
					}
					if (methodDef3.Name.StartsWith("InstallPack") && packs.ContainsKey(methodDef3.Name))
					{
						return packs[methodDef3.Name];
					}
				}
				if (methodDef3.Name == "RunInThread" && methodDef3.DeclaringType?.FullName == "PCL.ModBase")
				{
					return dstRunInThread2;
				}
				if (methodDef3.DeclaringType?.FullName == "PCL.ValidateBase" && methodDef3.Name == "Validate")
				{
					TypeDef typeDef5 = mod.Find("PCL.Validate", isReflectionName: false);
					if (typeDef5 != null)
					{
						foreach (MethodDef method5 in typeDef5.Methods)
						{
							if (method5.Name == "Validate" && method5.Parameters.Count == methodDef3.Parameters.Count && method5.IsVirtual == methodDef3.IsVirtual)
							{
								return method5;
							}
						}
					}
				}
				string text10 = methodDef3.DeclaringType?.FullName ?? "";
				TypeDef typeDef6 = null;
				if (methodDef3.DeclaringType != null)
				{
					typeDef6 = MapTypeStatic(methodDef3.DeclaringType, mod) as TypeDef;
				}
				if (typeDef6 == null && text10.Length > 0)
				{
					string text11 = text10;
					if (text11.StartsWith("VB_AnonymousDelegate_"))
					{
						string text12 = text11.Substring("VB_AnonymousDelegate_".Length);
						int num = text12.IndexOf('`');
						int num2 = text12.IndexOf('_');
						text11 = ((num2 <= 0 || (num >= 0 && num2 >= num)) ? ("VB$AnonymousDelegate_" + text12) : ("VB$AnonymousDelegate_" + text12.Substring(0, num2) + ((num >= 0) ? text12.Substring(num) : "")));
					}
					typeDef6 = mod.Find(text11, isReflectionName: false);
				}
				if (typeDef6 != null)
				{
					foreach (MethodDef method6 in typeDef6.Methods)
					{
						if (method6.Name == MapClosureMethodName(methodDef3.Name) && method6.Parameters.Count == methodDef3.Parameters.Count && method6.IsStatic == methodDef3.IsStatic)
						{
							return method6;
						}
					}
				}
				Console.WriteLine("WARN: fallback import " + methodDef3.FullName + " ttM=" + ((typeDef6 != null) ? (typeDef6.FullName + "(" + typeDef6.Methods.Count + ")") : "null"));
				return importer.Import(src);
			}
			if (src is MemberRef memberRef2)
			{
				if (memberRef2.Name == "RunInThread" && memberRef2.DeclaringType?.FullName == "PCL.ModBase")
				{
					return dstRunInThread2;
				}
				if (memberRef2.DeclaringType?.FullName == "PCL.ValidateBase" && memberRef2.Name == "Validate")
				{
					TypeDef typeDef7 = mod.Find("PCL.Validate", isReflectionName: false);
					if (typeDef7 != null)
					{
						foreach (MethodDef method7 in typeDef7.Methods)
						{
							if (method7.Name == "Validate" && method7.Parameters.Count == (memberRef2.MethodSig?.Params.Count ?? 0))
							{
								return method7;
							}
						}
					}
				}
				string text13 = memberRef2.DeclaringType?.FullName ?? "";
				string text14 = MapClosureTypeName(text13);
				if (text14 != text13 && text14.StartsWith("PCL."))
				{
					TypeDef typeDef8 = mod.Find(text14, isReflectionName: false);
					if (typeDef8 != null)
					{
						foreach (MethodDef method8 in typeDef8.Methods)
						{
							if (method8.Name == MapClosureMethodName(memberRef2.Name) && method8.Parameters.Count == (memberRef2.MethodSig?.Params.Count ?? 0))
							{
								return method8;
							}
						}
					}
				}
				if (text13.Contains("DisplayClass13_0") && dstClosure2 != null)
				{
					if (memberRef2.Name == ".ctor")
					{
						return dstClosureCtor2;
					}
					if (memberRef2.Name.Contains("b__0"))
					{
						return dstClosureLambda2;
					}
				}
				if (cloneMap2 != null && text13.StartsWith("PCL.") && cloneMap2.TryGetValue(text13, out var value8))
				{
					foreach (MethodDef method9 in value8.Methods)
					{
						if (method9.Name == memberRef2.Name)
						{
							return method9;
						}
					}
				}
				if (text13.Contains("_Closure___22_0") && dstC84 != null)
				{
					foreach (MethodDef method10 in dstC84.Methods)
					{
						if (method10.Name == memberRef2.Name)
						{
							return method10;
						}
					}
				}
				if (text13.StartsWith("PCL."))
				{
					TypeDef typeDef9 = mod.Find(MapClosureTypeName(text13), isReflectionName: false);
					if (typeDef9 != null)
					{
						foreach (MethodDef method11 in typeDef9.Methods)
						{
							if (method11.Name == MapClosureMethodName(memberRef2.Name) && method11.Parameters.Count == (memberRef2.MethodSig?.Params.Count ?? 0))
							{
								bool isStatic = method11.IsStatic;
								MethodSig methodSig2 = memberRef2.MethodSig;
								if (isStatic == (methodSig2 != null && !methodSig2.HasThis))
								{
									return method11;
								}
							}
						}
					}
				}
				if (memberRef2.DeclaringType is TypeSpec typeSpec3)
				{
					TypeSpecUser @class = new TypeSpecUser(RemapSigStatic(typeSpec3.TypeSig, mod));
					MemberRefUser memberRefUser2 = new MemberRefUser(mod, memberRef2.Name, RemapMethodSig(memberRef2.MethodSig), @class);
					if (memberRefUser2.MethodSig == null && memberRef2.FieldSig != null)
					{
						memberRefUser2.FieldSig = memberRef2.FieldSig;
					}
					return importer.Import(memberRefUser2);
				}
				ITypeDefOrRef typeDefOrRef6 = MapTypeStatic(memberRef2.DeclaringType, mod);
				if (typeDefOrRef6 != null && typeDefOrRef6 != memberRef2.DeclaringType)
				{
					MemberRefUser memberRefUser3 = new MemberRefUser(mod, memberRef2.Name, RemapMethodSig(memberRef2.MethodSig), typeDefOrRef6);
					if (memberRefUser3.MethodSig == null && memberRef2.FieldSig != null)
					{
						memberRefUser3.FieldSig = memberRef2.FieldSig;
					}
					return importer.Import(memberRefUser3);
				}
				return importer.Import(memberRef2);
			}
			return importer.Import(src);
		}
		FieldDef? ResolveField(FieldDef fd)
		{
			if (fd.DeclaringType != null && typeMap.TryGetValue(fd.DeclaringType, out var value4))
			{
				foreach (FieldDef field3 in value4.Fields)
				{
					if (field3.Name == fd.Name)
					{
						return field3;
					}
				}
			}
			if (fd.DeclaringType != null && cloneMap2 != null && fd.DeclaringType.FullName.StartsWith("PCL.") && cloneMap2.TryGetValue(fd.DeclaringType.FullName, out var value5))
			{
				foreach (FieldDef field4 in value5.Fields)
				{
					if (field4.Name == fd.Name)
					{
						return field4;
					}
				}
			}
			TypeDef declaringType = fd.DeclaringType;
			if (declaringType != null && (declaringType.FullName?.Contains("DisplayClass13_0")).GetValueOrDefault() && dstClosure2 != null)
			{
				if (dstTextField2 != null)
				{
					return dstTextField2;
				}
				foreach (FieldDef field5 in dstClosure2.Fields)
				{
					if (field5.Name == fd.Name)
					{
						return field5;
					}
				}
			}
			TypeDef typeDef = mod.Find(fd.DeclaringType?.FullName ?? "", isReflectionName: false);
			if (typeDef == null && fd.DeclaringType?.FullName != null)
			{
				string text3 = MapClosureTypeName(fd.DeclaringType.FullName);
				if (text3 != fd.DeclaringType.FullName)
				{
					typeDef = mod.Find(text3, isReflectionName: false);
				}
			}
			if (typeDef != null)
			{
				foreach (FieldDef field6 in typeDef.Fields)
				{
					if (field6.Name == fd.Name)
					{
						return field6;
					}
				}
			}
			string text4 = fd.Name?.ToString();
			if (fd.DeclaringType?.FullName != null && text4 != null)
			{
				string text5 = MapClosureTypeName(fd.DeclaringType.FullName);
				string prefix = text5.Substring(0, text5.LastIndexOf('/') + 1);
				string bare = text5.Substring(text5.LastIndexOf('/') + 1);
				string text6 = text4.Replace("_VB_Local_", "$VB$Local_").Replace("_VB_Me", "$VB$Me").Replace("_VB_NonLocal__VB_Closure_", "$VB$NonLocal__VB_Closure_");
				foreach (TypeDef item2 in from t2 in mod.GetTypes()
					where t2.Name == bare && t2.FullName.StartsWith(prefix)
					select t2)
				{
					foreach (FieldDef field7 in item2.Fields)
					{
						if (field7.Name == text6)
						{
							return field7;
						}
					}
				}
			}
			UTF8String uTF8String = fd.Name ?? ((UTF8String)"");
			if (uTF8String.StartsWith("<") && uTF8String.EndsWith(">k__BackingField"))
			{
				string text7 = "_" + uTF8String.Substring(1, uTF8String.Length - 1 - ">k__BackingField".Length);
				TypeDef typeDef2 = mod.Find(fd.DeclaringType?.FullName ?? "", isReflectionName: false);
				if (typeDef2 != null)
				{
					foreach (FieldDef field8 in typeDef2.Fields)
					{
						if (field8.Name == text7)
						{
							return field8;
						}
					}
				}
			}
			if (fd.DeclaringType != null && (fd.Name == "Output" || fd.DeclaringType.FullName.Contains("LoaderTask")))
			{
				Console.WriteLine(string.Concat("RFNULL ", fd.DeclaringType.FullName, "::", fd.Name, " aliasFound=", (mod.Find(fd.DeclaringType.FullName.Replace("LoaderTask_2`2", "LoaderTask`2"), isReflectionName: false) != null).ToString()));
			}
			return null;
		}
	}

	private static int Run(string input, string output, string refPath, string mode)
	{
		FullTransplant = mode == "uifull" || mode.Split(',').Contains("uifull") || mode.Split(',').Contains("noGR");
		if (!File.Exists(input))
		{
			return Fail("input not found: " + input);
		}
		ModuleDefMD mod;
		ModuleDefMD moduleDefMD;
		try
		{
			mod = ModuleDefMD.Load(input);
			moduleDefMD = ModuleDefMD.Load(refPath);
		}
		catch (Exception ex)
		{
			return Fail("failed to load assembly: " + ex.Message);
		}
		void CheckBody(MethodDef m)
		{
			if (!m.HasBody)
			{
				return;
			}
			foreach (Instruction instruction in m.Body.Instructions)
			{
				if (instruction.Operand is ITypeDefOrRef typeDefOrRef3 && typeDefOrRef3.Module != mod && typeDefOrRef3.Module != null)
				{
					instruction.Operand = mod.Import(typeDefOrRef3);
					Console.WriteLine(string.Concat("REIMPORT ", m.Name, " op=", instruction.OpCode?.ToString(), " -> ", typeDefOrRef3.FullName));
				}
				if (instruction.Operand is MemberRef { DeclaringType: not null } memberRef)
				{
					string fullName = memberRef.DeclaringType.FullName;
					if (fullName.Contains("PageDownloadResourceDetail/<>") || fullName.Contains("_Closure___22") || fullName.Contains("DisplayClass"))
					{
						Console.WriteLine(string.Concat("BADMR ", m.Name, " -> ", memberRef.FullName));
					}
				}
				if (instruction.Operand is FieldDef fieldDef3 && fieldDef3.Module != mod)
				{
					instruction.Operand = mod.Import(fieldDef3);
					Console.WriteLine(string.Concat("REIMPORT-FIELD ", m.Name, " -> ", fieldDef3.FullName));
				}
			}
		}
		TypeDef typeDef = mod.Find("PCL.ModModpack", isReflectionName: false);
		if (typeDef == null)
		{
			return Fail("PCL.ModModpack not found (not a supported PCL build)");
		}
		TypeDef typeDef2 = moduleDefMD.Find("PCL.ModModpack", isReflectionName: false);
		if (typeDef2 == null)
		{
			return Fail("reference image has no ModModpack");
		}
		MethodDef methodDef = null;
		MethodDef methodDef2 = null;
		foreach (MethodDef method5 in typeDef2.Methods)
		{
			if (method5.Name != "ModpackInstall")
			{
				continue;
			}
			if (method5.Parameters.Count <= 1)
			{
				if (methodDef == null)
				{
					methodDef = method5;
				}
			}
			else if (methodDef2 == null)
			{
				methodDef2 = method5;
			}
		}
		if (methodDef == null || !methodDef.HasBody)
		{
			return Fail("reference void ModpackInstall not found");
		}
		if (methodDef2 == null || !methodDef2.HasBody)
		{
			return Fail("reference param ModpackInstall not found");
		}
		TypeDef typeDef3 = moduleDefMD.Find("PCL.PageDownloadResourceDetail", isReflectionName: false);
		MethodDef methodDef3 = null;
		if (typeDef3 != null)
		{
			foreach (MethodDef method6 in typeDef3.Methods)
			{
				if (method6.Name == "Install_Click" && method6.HasBody)
				{
					methodDef3 = method6;
					break;
				}
			}
		}
		if (methodDef3 == null)
		{
			return Fail("reference Install_Click not found");
		}
		MethodDef methodDef4 = null;
		MethodDef methodDef5 = null;
		foreach (MethodDef method7 in typeDef.Methods)
		{
			if (method7.Name != "ModpackInstall")
			{
				continue;
			}
			if (method7.Parameters.Count <= 1)
			{
				if (methodDef4 == null)
				{
					methodDef4 = method7;
				}
			}
			else if (methodDef5 == null)
			{
				methodDef5 = method7;
			}
		}
		if (methodDef4 == null || methodDef5 == null)
		{
			return Fail("target ModpackInstall overloads not found");
		}
		bool num = methodDef4.HasBody && methodDef4.Body.Instructions.Count > 30;
		bool flag = methodDef5.HasBody && methodDef5.Body.Instructions.Count > 300;
		if (num && flag)
		{
			Console.WriteLine("SKIP: modpack install already restored");
			if (!File.Exists(output) || new FileInfo(input).Length != new FileInfo(output).Length)
			{
				File.Copy(input, output, overwrite: true);
				Console.WriteLine("Copied unchanged: " + output);
			}
			return 0;
		}
		Dictionary<string, IMethod> dictionary = new Dictionary<string, IMethod>();
		string[] array = new string[7] { "InstallPackCurseForge", "InstallPackHMCL", "InstallPackMMC", "InstallPackMCBBS", "InstallPackModrinth", "InstallPackLauncherPack", "InstallPackCompress" };
		foreach (string text in array)
		{
			IMethod method = null;
			foreach (MethodDef method8 in typeDef.Methods)
			{
				if (method8.Name == text)
				{
					method = method8;
					break;
				}
			}
			if (method == null)
			{
				return Fail("target InstallPack method missing: " + text);
			}
			dictionary[text] = method;
		}
		TypeDef typeDef4 = mod.Find("PCL.ModModpack/_Closure$__0-0", isReflectionName: false);
		MethodDef methodDef6 = typeDef4?.Methods.FirstOrDefault((MethodDef m2) => m2.Name == ".ctor");
		MethodDef methodDef7 = typeDef4?.Methods.FirstOrDefault((MethodDef m2) => m2.Name == "_Lambda$__0");
		FieldDef fieldDef = typeDef4?.Fields.FirstOrDefault((FieldDef f) => f.Name.Contains("File"));
		if (typeDef4 == null || methodDef6 == null || methodDef7 == null || fieldDef == null)
		{
			return Fail("target closure members not found");
		}
		FieldDef fieldDef2 = typeDef4.Fields.FirstOrDefault((FieldDef f2) => f2.Name == "$VB$Local_File") ?? typeDef4.Fields.FirstOrDefault((FieldDef f2) => f2.Name == "text");
		if (fieldDef2 == null)
		{
			fieldDef2 = new FieldDefUser("text", new FieldSig(mod.CorLibTypes.String), dnlib.DotNet.FieldAttributes.Public);
			typeDef4.Fields.Add(fieldDef2);
			Console.WriteLine("  added closure field 'text'");
		}
		IMethod method2 = null;
		TypeDef typeDef5 = mod.Find("PCL.ModBase", isReflectionName: false);
		if (typeDef5 != null)
		{
			foreach (MethodDef method9 in typeDef5.Methods)
			{
				if (method9.Name == "RunInThread" && method9.MethodSig != null && method9.MethodSig.Params.Count == 1)
				{
					method2 = method9;
					break;
				}
			}
		}
		if (method2 == null)
		{
			return Fail("PCL.ModBase.RunInThread not found");
		}
		IMethod method3 = null;
		foreach (MemberRef memberRef2 in mod.GetMemberRefs())
		{
			if (memberRef2.Name != "op_Implicit")
			{
				continue;
			}
			string text2 = memberRef2.DeclaringType?.FullName ?? "";
			if (text2.Contains("OneOf") && text2.Contains("ValueTuple"))
			{
				MethodSig methodSig = memberRef2.MethodSig;
				if (methodSig != null && methodSig.Params.Count == 1 && methodSig.Params[0] is GenericVar { Number: 0u })
				{
					method3 = memberRef2;
					break;
				}
			}
		}
		if (method3 == null)
		{
			return Fail("OneOf T1 op_Implicit not found");
		}
		Importer importer = new Importer(mod, ImporterOptions.TryToUseDefs);
		TypeRefUser baseType = new TypeRefUser(mod, "System", "Object", mod.CorLibTypes.AssemblyRef);
		Dictionary<TypeDef, TypeDef> dictionary2 = new Dictionary<TypeDef, TypeDef>();
		Dictionary<string, TypeDef> dictionary3 = new Dictionary<string, TypeDef>();
		dictionary3["PCL.ModModpack/<>c__DisplayClass13_0"] = typeDef4;
		TypeDef typeDef6 = mod.Find("PCL.PageDownloadResourceDetail", isReflectionName: false);
		TypeDef dstC = null;
		if (typeDef6 == null)
		{
			return Fail("target detail page not found");
		}
		TypeDef typeDef7 = (mode.Contains("nooh") ? null : moduleDefMD.Find("PCL.OneOfHelper", isReflectionName: false));
		if (typeDef7 != null && mod.Find("PCL.OneOfHelper", isReflectionName: false) == null)
		{
			TypeDefUser typeDefUser = new TypeDefUser("PCL", "OneOfHelper");
			typeDefUser.Attributes = typeDef7.Attributes;
			typeDefUser.BaseType = baseType;
			dictionary3["PCL.OneOfHelper"] = typeDefUser;
			List<(MethodDef, MethodDefUser)> list = new List<(MethodDef, MethodDefUser)>();
			foreach (FieldDef field in typeDef7.Fields)
			{
				typeDefUser.Fields.Add(new FieldDefUser(field.Name, new FieldSig(RemapSigStatic(field.FieldSig.Type, mod)), field.Attributes));
			}
			foreach (MethodDef method10 in typeDef7.Methods)
			{
				MethodSig methodSig2 = ((method10.MethodSig == null) ? null : RemapMethodSigStatic(method10.MethodSig, mod));
				if (methodSig2 != null && method10.MethodSig != null && method10.MethodSig.Generic && method10.MethodSig.GenParamCount != 0)
				{
					methodSig2.GenParamCount = method10.MethodSig.GenParamCount;
				}
				List<GenericParamUser> list2 = new List<GenericParamUser>();
				foreach (GenericParam genericParameter in method10.GenericParameters)
				{
					GenericParamUser genericParamUser = new GenericParamUser(genericParameter.Number, genericParameter.Flags, genericParameter.Name);
					foreach (GenericParamConstraint genericParamConstraint in genericParameter.GenericParamConstraints)
					{
						genericParamUser.GenericParamConstraints.Add(new GenericParamConstraintUser(RemapSigStatic(genericParamConstraint.Constraint.ToTypeSig(), mod).ToTypeDefOrRef()));
					}
					list2.Add(genericParamUser);
				}
				MethodDefUser methodDefUser = new MethodDefUser(method10.Name, methodSig2, method10.ImplAttributes, method10.Attributes);
				foreach (GenericParamUser item in list2)
				{
					methodDefUser.GenericParameters.Add(item);
				}
				typeDefUser.Methods.Add(methodDefUser);
				list.Add((method10, methodDefUser));
			}
			foreach (var (srcMethod, dstMethod) in list)
			{
				CopyBody(srcMethod, dstMethod, mod, importer, methodDef5, dictionary, method3, method2, dictionary2, typeDef3, typeDef6, isFwd: false, typeDef4, methodDef6, methodDef7, fieldDef, null, dstC, dictionary3, fieldDef2);
			}
			mod.Types.Add(typeDefUser);
			Console.WriteLine("  OneOfHelper cloned OK");
		}
		TypeDef typeDef8 = ((mode == "nopage" || mode.Contains("nopagec")) ? null : typeDef3.NestedTypes.FirstOrDefault((TypeDef t) => t.Name == "<>c"));
		if (typeDef8 != null)
		{
			TypeDefUser typeDefUser2 = new TypeDefUser("<>c");
			typeDefUser2.Attributes = typeDef8.Attributes;
			typeDefUser2.BaseType = baseType;
			dictionary3["PCL.PageDownloadResourceDetail/<>c"] = typeDefUser2;
			dictionary2[typeDef8] = typeDefUser2;
			List<(MethodDef, MethodDefUser)> list3 = new List<(MethodDef, MethodDefUser)>();
			foreach (FieldDef field2 in typeDef8.Fields)
			{
				TypeSig type = field2.FieldSig.Type;
				if (type is ClassSig classSig && classSig.TypeDefOrRef == typeDef8)
				{
					typeDefUser2.Fields.Add(new FieldDefUser(field2.Name, new FieldSig(new ClassSig(typeDefUser2)), field2.Attributes));
				}
				else
				{
					typeDefUser2.Fields.Add(new FieldDefUser(field2.Name, new FieldSig(RemapSigStatic(type, mod)), field2.Attributes));
				}
			}
			foreach (MethodDef method11 in typeDef8.Methods)
			{
				MethodSig methodSig3 = ((method11.MethodSig == null) ? null : RemapMethodSigStatic(method11.MethodSig, mod));
				MethodDefUser methodDefUser2 = new MethodDefUser(method11.Name, (method11.Name == ".ctor") ? MethodSig.CreateInstance(mod.CorLibTypes.Void) : methodSig3, method11.ImplAttributes, method11.Attributes);
				typeDefUser2.Methods.Add(methodDefUser2);
				list3.Add((method11, methodDefUser2));
			}
			foreach (var (srcMethod2, dstMethod2) in list3)
			{
				CopyBody(srcMethod2, dstMethod2, mod, importer, methodDef5, dictionary, method3, method2, dictionary2, typeDef3, typeDef6, isFwd: false, typeDef4, methodDef6, methodDef7, fieldDef, null, dstC, dictionary3, fieldDef2);
			}
			typeDef6.NestedTypes.Add(typeDefUser2);
			Console.WriteLine("  page <>c cloned OK");
		}
		TypeDef typeDef9 = typeDef3.NestedTypes.FirstOrDefault((TypeDef t) => t.Name == "<>c__DisplayClass83_0");
		if (typeDef9 != null && typeDef6.NestedTypes.All((TypeDef t) => t.Name != "<>c__DisplayClass83_0"))
		{
			TypeDefUser typeDefUser3 = new TypeDefUser("<>c__DisplayClass83_0");
			typeDefUser3.Attributes = typeDef9.Attributes;
			typeDefUser3.BaseType = baseType;
			dictionary3["PCL.PageDownloadResourceDetail/<>c__DisplayClass83_0"] = typeDefUser3;
			dictionary3["/<>c__DisplayClass83_0"] = typeDefUser3;
			dictionary3["<>c__DisplayClass83_0"] = typeDefUser3;
			dictionary2[typeDef9] = typeDefUser3;
			List<(MethodDef, MethodDefUser)> list4 = new List<(MethodDef, MethodDefUser)>();
			foreach (FieldDef field3 in typeDef9.Fields)
			{
				TypeSig type2 = field3.FieldSig.Type;
				if (type2.FullName.Contains("22_0") || type2.FullName.Contains("22-0"))
				{
					typeDefUser3.Fields.Add(new FieldDefUser(field3.Name, new FieldSig(RemapSigStatic(type2, mod)), field3.Attributes));
				}
				else
				{
					typeDefUser3.Fields.Add(new FieldDefUser(field3.Name, new FieldSig(RemapSigStatic(type2, mod)), field3.Attributes));
				}
			}
			foreach (MethodDef method12 in typeDef9.Methods)
			{
				MethodSig methodSig4 = ((method12.Name == ".ctor") ? MethodSig.CreateInstance(mod.CorLibTypes.Void) : ((method12.MethodSig == null) ? null : RemapMethodSigStatic(method12.MethodSig, mod)));
				MethodDefUser methodDefUser3 = new MethodDefUser(method12.Name, methodSig4, method12.ImplAttributes, method12.Attributes);
				typeDefUser3.Methods.Add(methodDefUser3);
				list4.Add((method12, methodDefUser3));
			}
			typeDef6.NestedTypes.Add(typeDefUser3);
			dstC = typeDefUser3;
			foreach (var (srcMethod3, dstMethod3) in list4)
			{
				CopyBody(srcMethod3, dstMethod3, mod, importer, methodDef5, dictionary, method3, method2, dictionary2, typeDef3, typeDef6, isFwd: false, typeDef4, methodDef6, methodDef7, fieldDef, null, dstC, dictionary3, fieldDef2);
			}
			Console.WriteLine("  stable 83_0 cloned");
		}
		TypeDef typeDef10 = mod.Find("PCL.PageResource", isReflectionName: false);
		TypeDef typeDef11 = moduleDefMD.Find("PCL.PageResource", isReflectionName: false);
		if (typeDef10 != null && typeDef11 != null)
		{
			MethodDef methodDef8 = typeDef10.FindMethod("set_PageType");
			MethodDef methodDef9 = typeDef11.FindMethod("set_PageType");
			if (methodDef8 != null && methodDef9 != null && methodDef9.HasBody)
			{
				CopyBody(methodDef9, methodDef8, mod, importer, methodDef5, dictionary, method3, method2, dictionary2, typeDef3, typeDef6, isFwd: false, typeDef4, methodDef6, methodDef7, fieldDef, null, dstC, dictionary3, fieldDef2);
				Console.WriteLine("  PageResource.set_PageType overwritten with 2.13 visibility logic");
			}
		}
		if (mode == "noinstall")
		{
			Console.WriteLine("SKIP install");
		}
		else
		{
			CopyBody(methodDef, methodDef4, mod, importer, methodDef5, dictionary, method3, method2, dictionary2, typeDef3, typeDef6, isFwd: false, typeDef4, methodDef6, methodDef7, fieldDef, null, dstC, dictionary3, fieldDef2);
		}
		Console.WriteLine("Void transplanted: " + methodDef4.Body.Instructions.Count + " instructions");
		CopyBody(methodDef2, methodDef5, mod, importer, methodDef5, dictionary, method3, method2, dictionary2, typeDef3, typeDef6, isFwd: false, typeDef4, methodDef6, methodDef7, fieldDef, null, dstC, dictionary3, fieldDef2);
		Console.WriteLine("Str transplanted: " + methodDef5.Body.Instructions.Count + " instructions");
		ITypeDefOrRef typeDefOrRef = mod.Find("PCL.MyListItem", isReflectionName: false);
		ITypeDefOrRef typeDefOrRef2 = mod.Find("System.Windows.Input.MouseButtonEventArgs", isReflectionName: false);
		if (typeDefOrRef == null)
		{
			foreach (TypeRef typeRef in mod.GetTypeRefs())
			{
				if (typeRef.FullName == "PCL.MyListItem")
				{
					typeDefOrRef = typeRef;
					break;
				}
			}
		}
		if (typeDefOrRef2 == null)
		{
			foreach (TypeRef typeRef2 in mod.GetTypeRefs())
			{
				if (typeRef2.FullName == "System.Windows.Input.MouseButtonEventArgs")
				{
					typeDefOrRef2 = typeRef2;
					break;
				}
			}
		}
		if (typeDefOrRef == null || typeDefOrRef2 == null)
		{
			return Fail("detail forward types not found");
		}
		MethodDefUser methodDefUser4 = new MethodDefUser("InstallForward_Click", MethodSig.CreateInstance(mod.CorLibTypes.Void, typeDefOrRef.ToTypeSig(), typeDefOrRef2.ToTypeSig()), dnlib.DotNet.MethodImplAttributes.IL, dnlib.DotNet.MethodAttributes.Public | dnlib.DotNet.MethodAttributes.HideBySig);
		if (mode != "stubfwd")
		{
			CopyBody(methodDef3, methodDefUser4, mod, importer, methodDef5, dictionary, method3, method2, dictionary2, typeDef3, typeDef6, isFwd: true, typeDef4, methodDef6, methodDef7, fieldDef, null, dstC, dictionary3, fieldDef2);
		}
		else
		{
			methodDefUser4.Body = new CilBody();
			methodDefUser4.Body.MaxStack = 8;
			methodDefUser4.Body.Instructions.Add(Instruction.Create(OpCodes.Ret));
		}
		typeDef6.Methods.Add(methodDefUser4);
		if (mode.Contains("nodisp"))
		{
			Console.WriteLine("SKIP dispatcher/fwd");
			return finishRun(mod, output);
		}
		typeDef.Methods.FirstOrDefault((MethodDef m2) => m2.Name == "ModpackInstall" && m2.Parameters.Count <= 1);
		MethodDef methodSave = saveClickOrNull(typeDef6);
		if (methodSave == null)
		{
			return Fail("target Save_Click not found");
		}
		TypeDef typeDefResourceVersion = mod.Find("PCL.ResourceVersion", isReflectionName: false);
		if (typeDefResourceVersion == null)
		{
			return Fail("target ResourceVersion not found");
		}
		FieldDef fieldResourceType = typeDefResourceVersion.FindField("ResourceType");
		if (fieldResourceType == null)
		{
			return Fail("target ResourceVersion.ResourceType not found");
		}
		IMethod getTag = null;
		foreach (MemberRef memberRef3 in mod.GetMemberRefs())
		{
			if (memberRef3.Name == "get_Tag" && memberRef3.DeclaringType != null && memberRef3.DeclaringType.FullName == "System.Windows.FrameworkElement")
			{
				getTag = memberRef3;
				break;
			}
		}
		if (getTag == null)
		{
			return Fail("FrameworkElement.get_Tag not found");
		}
		MethodDefUser methodDefUser6 = new MethodDefUser("SaveForward_Click", MethodSig.CreateInstance(mod.CorLibTypes.Void, mod.CorLibTypes.Object, typeDefOrRef2.ToTypeSig()), dnlib.DotNet.MethodImplAttributes.IL, dnlib.DotNet.MethodAttributes.Public | dnlib.DotNet.MethodAttributes.HideBySig);
		methodDefUser6.Body = new CilBody();
		methodDefUser6.Body.MaxStack = 8;
		methodDefUser6.Body.InitLocals = false;
		IList<Instruction> instructions6 = methodDefUser6.Body.Instructions;
		instructions6.Add(Instruction.Create(OpCodes.Ldarg_0));
		instructions6.Add(Instruction.Create(OpCodes.Ldarg_1));
		instructions6.Add(Instruction.Create(OpCodes.Ldnull));
		instructions6.Add(Instruction.Create(OpCodes.Callvirt, methodSave));
		instructions6.Add(Instruction.Create(OpCodes.Ret));
		typeDef6.Methods.Add(methodDefUser6);		MethodDefUser methodDefUser5 = new MethodDefUser("InstallOrSave_Click", MethodSig.CreateInstance(mod.CorLibTypes.Void, mod.CorLibTypes.Object, typeDefOrRef2.ToTypeSig()), dnlib.DotNet.MethodImplAttributes.IL, dnlib.DotNet.MethodAttributes.Public | dnlib.DotNet.MethodAttributes.HideBySig);
		methodDefUser5.Body = new CilBody();
		methodDefUser5.Body.MaxStack = 8;
		methodDefUser5.Body.InitLocals = true;
		methodDefUser5.Body.Variables.Add(new dnlib.DotNet.Emit.Local(new ClassSig(typeDefResourceVersion)));
		IList<Instruction> instructions = methodDefUser5.Body.Instructions;
		Instruction branchNotModpack = Instruction.Create(OpCodes.Brfalse, Instruction.Create(OpCodes.Nop));
		Instruction branchSave = Instruction.Create(OpCodes.Bne_Un, Instruction.Create(OpCodes.Nop));
		Instruction savePath = Instruction.Create(OpCodes.Ldarg_0);
		instructions.Add(Instruction.Create(OpCodes.Ldarg_0));
		instructions.Add(Instruction.Create(OpCodes.Ldarg_1));
		instructions.Add(Instruction.Create(OpCodes.Castclass, typeDefOrRef));
		instructions.Add(Instruction.Create(OpCodes.Callvirt, getTag));
		instructions.Add(Instruction.Create(OpCodes.Isinst, typeDefResourceVersion));
		instructions.Add(Instruction.Create(OpCodes.Stloc_0));
		instructions.Add(Instruction.Create(OpCodes.Ldloc_0));
		instructions.Add(branchNotModpack);
		instructions.Add(Instruction.Create(OpCodes.Ldloc_0));
		instructions.Add(Instruction.Create(OpCodes.Ldfld, fieldResourceType));
		instructions.Add(Instruction.Create(OpCodes.Ldc_I4_2));
		instructions.Add(branchSave);
		instructions.Add(Instruction.Create(OpCodes.Ldarg_0));
		instructions.Add(Instruction.Create(OpCodes.Ldarg_1));
		instructions.Add(Instruction.Create(OpCodes.Castclass, typeDefOrRef));
		instructions.Add(Instruction.Create(OpCodes.Ldarg_2));
		instructions.Add(Instruction.Create(OpCodes.Callvirt, methodDefUser4));
		instructions.Add(Instruction.Create(OpCodes.Ret));
		instructions.Add(savePath);
		instructions.Add(Instruction.Create(OpCodes.Ldarg_1));
		instructions.Add(Instruction.Create(OpCodes.Ldarg_2));
		instructions.Add(Instruction.Create(OpCodes.Callvirt, methodDefUser6));
		instructions.Add(Instruction.Create(OpCodes.Ret));
		branchNotModpack.Operand = savePath;
		branchSave.Operand = savePath;
		typeDef6.Methods.Add(methodDefUser5);
		MethodDef methodDef10 = typeDef6.FindMethod("InitializeStackResourceVersions");
		if (methodDef10 != null && methodDef10.HasBody)
		{
			foreach (Instruction instruction2 in methodDef10.Body.Instructions)
			{
				MethodDef methodDef11 = instruction2.Operand as MethodDef;
				if (instruction2.OpCode.Code == Code.Ldftn && methodDef11 != null && methodDef11.Name == "Save_Click" && methodDef11.DeclaringType != null && methodDef11.DeclaringType.FullName == "PCL.PageDownloadResourceDetail")
				{
					instruction2.Operand = methodDefUser5;
					Console.WriteLine("  Rebound: version list item click → InstallOrSave_Click");
				}
			}
		}
		TypeDef typeDef12 = mod.Find("PCL.MyIconButton", isReflectionName: false);
		TypeDef typeDef13 = mod.Find("PCL.ResourceVersion", isReflectionName: false);
		if (typeDef12 != null && typeDef13 != null)
		{
			TypeDef typeDef14 = typeDef12.NestedTypes.FirstOrDefault((TypeDef t) => t.Name.ToString() == "ClickEventHandler");
			TypeDef typeDef15 = typeDef13.NestedTypes.FirstOrDefault((TypeDef t) => t.Name.ToString() == "_Closure$__27-0");
			MethodDef methodDef12 = typeDef13.FindMethod("ToListItem");
			if (typeDef14 != null && typeDef15 != null && methodDef12 != null)
			{
				MethodDef method4 = typeDef6.FindMethod("Save_Click");
				MemberRefUser mr = new MemberRefUser(mod, ".ctor", MethodSig.CreateInstance(mod.CorLibTypes.Void, mod.CorLibTypes.Object, mod.CorLibTypes.IntPtr), typeDef14);
				MethodDef methodDef13 = typeDef6.FindMethod("InitializeStackResourceVersions");
				if (methodDef13 != null && methodDef13.HasBody && !methodDef13.IsStatic)
				{
					IList<Instruction> instructions2 = methodDef13.Body.Instructions;
					for (int j = 0; j < instructions2.Count - 2; j++)
					{
						MethodDef methodDef14 = instructions2[j].Operand as MethodDef;
						if (instructions2[j].OpCode.Code == Code.Ldftn && methodDef14 != null && !(methodDef14.Name.ToString() != "InstallOrSave_Click") && instructions2[j + 1].OpCode.Code == Code.Newobj && instructions2[j + 2].OpCode.Code == Code.Ldnull)
						{
							instructions2[j + 2].OpCode = OpCodes.Ldarg_0;
							instructions2.Insert(j + 3, Instruction.Create(OpCodes.Ldftn, method4));
							instructions2.Insert(j + 4, Instruction.Create(OpCodes.Newobj, mr));
							methodDef13.Body.MaxStack = 16;
							Console.WriteLine("  Restored: version row save-as button → Save_Click");
							break;
						}
					}
				}
			}
		}
		Console.WriteLine("Detail page click restored");
		CheckBody(methodDef4);
		CheckBody(methodDef5);
		CheckBody(methodDefUser4);
		CheckBody(methodDefUser5);
		foreach (TypeDef nestedType in typeDef6.NestedTypes)
		{
			foreach (MethodDef method13 in nestedType.Methods)
			{
				CheckBody(method13);
			}
		}
		CheckBody(typeDef6.FindMethod("PageDownloadResourceDetail_Inited") ?? typeDef6.Methods.First((MethodDef m3) => m3.Name == "PageDownloadResourceDetail_Inited"));
		IEnumerable<MethodDef> enumerable = mod.Find("PCL.OneOfHelper", isReflectionName: false)?.Methods;
		foreach (MethodDef item2 in enumerable ?? Enumerable.Empty<MethodDef>())
		{
			CheckBody(item2);
		}
		return finishRun(mod, output);
	}

	private static int finishRun(ModuleDef mod, string output)
	{
		try
		{
			ModuleWriterOptions moduleWriterOptions = new ModuleWriterOptions(mod);
			moduleWriterOptions.MetadataOptions.Flags |= MetadataFlags.KeepOldMaxStack;
			moduleWriterOptions.MetadataLogger = new ErrLogger();
			mod.Write(output, moduleWriterOptions);
		}
		catch (Exception ex)
		{
			foreach (MemberRef memberRef in mod.GetMemberRefs())
			{
				if (memberRef.Signature == null)
				{
					Console.WriteLine("NULLSIGREF " + memberRef.FullName + " decl=" + (memberRef.DeclaringType?.FullName ?? "?") + " isUser=" + (memberRef is MemberRefUser));
				}
			}
			foreach (TypeDef type in mod.GetTypes())
			{
				foreach (MethodDef method in type.Methods)
				{
					if (method.MethodSig == null)
					{
						Console.WriteLine("NULLMETHODSIG " + type.FullName + "::" + method.Name);
					}
				}
			}
			return Fail("failed to write output: " + ex.Message);
		}
		Action<string, TypeSig> wsig = null;
		wsig = delegate(string where, TypeSig sig)
		{
			if (sig != null)
			{
				ITypeDefOrRef typeDefOrRef = sig.TryGetTypeDefOrRef();
				if (typeDefOrRef != null && typeDefOrRef.Module != null && typeDefOrRef.Module != mod && typeDefOrRef is TypeDef)
				{
					Console.WriteLine("LEAKOK " + where + " -> " + sig.FullName);
				}
				if (sig is GenericInstSig genericInstSig)
				{
					wsig(where + "/o", genericInstSig.GenericType);
					foreach (TypeSig genericArgument in genericInstSig.GenericArguments)
					{
						wsig(where + "/a", genericArgument);
					}
				}
				if (sig.Next != null)
				{
					wsig(where + "/n", sig.Next);
				}
			}
		};
		foreach (TypeDef type2 in mod.GetTypes())
		{
			foreach (FieldDef field in type2.Fields)
			{
				if (field.FieldSig != null)
				{
					wsig(string.Concat("f ", type2.Name, "::", field.Name), field.FieldSig.Type);
				}
			}
			foreach (MethodDef method2 in type2.Methods)
			{
				if (method2.MethodSig != null)
				{
					wsig("m " + method2.FullName, method2.MethodSig.RetType);
				}
				if (!method2.HasBody)
				{
					continue;
				}
				foreach (Local variable in method2.Body.Variables)
				{
					wsig("l " + method2.Name, variable.Type);
				}
			}
		}
		Console.WriteLine("Saved: " + output);
		return 0;
	}

	private static FieldDef? targetTypeOrNull(TypeDef page)
	{
		foreach (FieldDef field in page.Fields)
		{
			if (field.Name == "TargetResourceType")
			{
				return field;
			}
		}
		return null;
	}

	private static MethodDef? saveClickOrNull(TypeDef page)
	{
		foreach (MethodDef method in page.Methods)
		{
			if (method.Name == "Save_Click")
			{
				return method;
			}
		}
		return null;
	}

	private static int Main(string[] args)
	{
		if (args.Length < 2)
		{
			Console.WriteLine("ERROR: usage: ilpatch <input exe> <output exe>");
			return 1;
		}
		string input = args[0];
		string output = args[1];
		string text = Path.Combine(Path.GetTempPath(), "ilpatch_ref_" + Guid.NewGuid().ToString("N").Substring(0, 8) + ".exe");
		try
		{
			using Stream stream = typeof(Program).Assembly.GetManifestResourceStream("ilpatch.ref_pcl.bin");
			if (stream == null)
			{
				Console.WriteLine("ERROR: embedded reference image not found");
				return 1;
			}
			using FileStream destination = File.Create(text);
			stream.CopyTo(destination);
		}
		catch (Exception ex)
		{
			Console.WriteLine("ERROR: failed to extract embedded reference: " + ex.Message);
			return 1;
		}
		try
		{
			return Run(input, output, text, (args.Length > 2) ? args[2] : "");
		}
		catch (Exception ex2)
		{
			Console.WriteLine("ERROR: " + ex2.GetType().Name + ": " + ex2.Message);
			return 1;
		}
		finally
		{
			try
			{
				if (File.Exists(text))
				{
					File.Delete(text);
				}
			}
			catch
			{
			}
		}
	}
}
