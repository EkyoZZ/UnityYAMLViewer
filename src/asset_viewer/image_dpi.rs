//! Ask the same Windows imaging codec family for density, without writing the image.
#[cfg(not(windows))]
pub(super) fn windows_resolution(_bytes:&[u8])->Option<(f64,f64)>{None}
#[cfg(windows)]
pub(super) fn windows_resolution(bytes:&[u8])->Option<(f64,f64)>{
    use std::ffi::c_void;
    #[repr(C)]struct Guid{a:u32,b:u16,c:u16,d:[u8;8]}
    #[link(name="ole32")]unsafe extern "system"{
        fn CoInitializeEx(reserved:*mut c_void,mode:u32)->i32;
        fn CoUninitialize();
        fn CoCreateInstance(class:*const Guid,outer:*mut c_void,context:u32,iid:*const Guid,result:*mut *mut c_void)->i32;
    }
    #[link(name="shlwapi")]unsafe extern "system"{fn SHCreateMemStream(bytes:*const u8,size:u32)->*mut c_void;}
    unsafe fn method(object:*mut c_void,index:usize)->*const c_void{unsafe{let table=*(object as *const *const *const c_void);*table.add(index)}}
    struct Com(*mut c_void);impl Drop for Com{fn drop(&mut self){if !self.0.is_null(){unsafe{let release:unsafe extern "system" fn(*mut c_void)->u32=std::mem::transmute(method(self.0,2));release(self.0);}}}}
    struct Apartment(bool);impl Drop for Apartment{fn drop(&mut self){if self.0{unsafe{CoUninitialize();}}}}
    let size=u32::try_from(bytes.len()).ok()?;
    unsafe{
        let initialized=CoInitializeEx(std::ptr::null_mut(),0);
        if initialized<0&&initialized as u32!=0x80010106{return None;}
        let _apartment=Apartment(initialized>=0);
        let class=Guid{a:0xcacaf262,b:0x9370,c:0x4615,d:[0xa1,0x3b,0x9f,0x55,0x39,0xda,0x4c,0x0a]};
        let iid=Guid{a:0xec5ec8a9,b:0xc395,c:0x4314,d:[0x9c,0x77,0x54,0xd7,0xa9,0x35,0xff,0x70]};
        let mut factory=Com(std::ptr::null_mut());if CoCreateInstance(&class,std::ptr::null_mut(),1,&iid,&mut factory.0)<0||factory.0.is_null(){return None;}
        let stream=Com(SHCreateMemStream(bytes.as_ptr(),size));if stream.0.is_null(){return None;}
        let create:unsafe extern "system" fn(*mut c_void,*mut c_void,*const Guid,u32,*mut *mut c_void)->i32=std::mem::transmute(method(factory.0,4));
        let mut decoder=Com(std::ptr::null_mut());if create(factory.0,stream.0,std::ptr::null(),0,&mut decoder.0)<0||decoder.0.is_null(){return None;}
        let frame:unsafe extern "system" fn(*mut c_void,u32,*mut *mut c_void)->i32=std::mem::transmute(method(decoder.0,13));
        let mut image=Com(std::ptr::null_mut());if frame(decoder.0,0,&mut image.0)<0||image.0.is_null(){return None;}
        let resolution:unsafe extern "system" fn(*mut c_void,*mut f64,*mut f64)->i32=std::mem::transmute(method(image.0,5));
        let (mut x,mut y)=(0.0,0.0);if resolution(image.0,&mut x,&mut y)<0||!x.is_finite()||!y.is_finite()||x<=0.0||y<=0.0{return None;}Some((x,y))
    }
}
