use crate::tables::*;
use crate::types::*;
use crate::TypeReader;

use proc_macro2::TokenStream;
use quote::quote;

#[derive(Debug)]
pub struct Delegate {
    pub name: TypeName,
    pub method: Method,
    pub guid: TypeGuid,
}

impl Delegate {
    pub fn from_type_def(reader: &TypeReader, def: TypeDef) -> Self {
        let name = TypeName::from_type_def(reader, def);
        let method = def
            .methods(reader)
            .find(|method| method.name(reader) == "Invoke")
            .unwrap();
        let method = Method::from_method_def(reader, method, &name.generics);
        let guid = TypeGuid::from_type_def(reader, def);
        Self { name, method, guid }
    }

    pub fn dependencies(&self) -> Vec<TypeDef> {
        self.method.dependencies()
    }

    pub fn to_tokens(&self) -> TokenStream {
        let definition = self.name.to_definition_tokens(&self.name.namespace);
        let abi_definition = self.name.to_abi_definition_tokens(&self.name.namespace);
        let impl_definition = self.to_impl_definition_tokens();
        let name = self.name.to_tokens(&self.name.namespace);
        let phantoms = self.name.phantoms();
        let constraints = self.name.constraints();
        let method = self.method.to_default_tokens(&self.name.namespace);
        let abi_method = self.method.to_abi_tokens(&self.name, &self.name.namespace);
        let guid = self.guid.to_tokens();

        quote! {
            #[repr(transparent)]
            #[derive(Default)]
            pub struct #definition where #constraints {
                ptr: ::winrt::ComPtr<#name>,
                #phantoms
            }
            impl<#constraints> #name {
                #method
            }
            unsafe impl<#constraints> ::winrt::ComInterface for #name {
                type VTable = #abi_definition;
                const IID: ::winrt::Guid = ::winrt::Guid::from_values(#guid);
            }
            impl<#constraints> ::std::clone::Clone for #name {
                fn clone(&self) -> Self {
                    Self {
                        ptr: self.ptr.clone(),
                        #phantoms
                    }
                }
            }
            #[repr(C)]
            pub struct #abi_definition where #constraints {
                pub unknown_query_interface: extern "system" fn(::winrt::RawComPtr<::winrt::IUnknown>, &::winrt::Guid, *mut ::winrt::RawPtr) -> ::winrt::ErrorCode,
                pub unknown_add_ref: extern "system" fn(::winrt::RawComPtr<::winrt::IUnknown>) -> u32,
                pub unknown_release: extern "system" fn(::winrt::RawComPtr<::winrt::IUnknown>) -> u32,
                #abi_method
                #phantoms
            }
            unsafe impl<#constraints> ::winrt::RuntimeType for #name {
                type Abi = ::winrt::RawComPtr<Self>;
                fn abi(&self) -> Self::Abi {
                    <::winrt::ComPtr<Self> as ::winrt::ComInterface>::as_raw(&self.ptr)
                }
                fn set_abi(&mut self) -> *mut Self::Abi {
                    self.ptr.set_abi()
                }
            }
            #[repr(C)]
            struct #impl_definition where #constraints {
                vtable: *const #abi_definition,
                count: ::winrt::RefCount,
                invoke: F,
            }
        }
    }

    pub fn to_impl_definition_tokens(&self) -> TokenStream {
        if self.name.generics.is_empty() {
            let name = format_impl_ident(&self.name.name);
            quote! { #name<F: FnMut() -> ::winrt::Result<()>> }
        } else {
            let name = format_impl_ident(&self.name.name[..self.name.name.len() - 2]);
            let generics = self.name.generics.iter().map(|g| g.to_tokens(&self.name.namespace));
            quote! { #name<#(#generics,)* F: FnMut() -> ::winrt::Result<()>> }
        }
    }
}

fn format_impl_ident(name: &str) -> proc_macro2::Ident {
    quote::format_ident!("impl_{}", name)
}
