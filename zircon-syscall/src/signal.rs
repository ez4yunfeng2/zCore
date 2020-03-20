use {
    super::*,
    core::sync::atomic::*,
    core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
    zircon_object::{signal::*, task::PolicyCondition},
};

impl Syscall<'_> {
    pub fn sys_port_create(
        &self,
        options: u32,
        mut out: UserOutPtr<HandleValue>,
    ) -> ZxResult<usize> {
        info!("port.create: options = {:#x}", options);
        if options != 0 {
            unimplemented!()
        }
        let port_handle = Handle::new(Port::new(), Rights::DEFAULT_PORT);
        let handle_value = self.thread.proc().add_handle(port_handle);
        out.write(handle_value)?;
        Ok(0)
    }

    pub fn sys_timer_create(
        &self,
        options: u32,
        clock_id: u32,
        mut out: UserOutPtr<HandleValue>,
    ) -> ZxResult<usize> {
        info!(
            "timer.create: options = {:#x}, clock_id={:#x}",
            options, clock_id
        );
        if clock_id != 0 {
            return Err(ZxError::INVALID_ARGS);
        }
        let proc = self.thread.proc();
        proc.check_policy(PolicyCondition::NewTimer)?;
        let handle = Handle::new(Timer::create(options)?, Rights::DEFAULT_TIMER);
        out.write(proc.add_handle(handle))?;
        Ok(0)
    }

    pub fn sys_event_create(
        &self,
        options: u32,
        mut out: UserOutPtr<HandleValue>,
    ) -> ZxResult<usize> {
        info!("event.create: options = {:#x}", options);
        if options != 0 {
            return Err(ZxError::INVALID_ARGS);
        }
        let proc = self.thread.proc();
        proc.check_policy(PolicyCondition::NewEvent)?;
        let handle = Handle::new(Event::new(), Rights::DEFAULT_EVENT);
        out.write(proc.add_handle(handle))?;
        Ok(0)
    }

    pub async fn sys_port_wait(
        &self,
        handle_value: HandleValue,
        deadline: u64,
        mut packet_res: UserOutPtr<PortPacket>,
    ) -> ZxResult<usize> {
        info!(
            "port.wait: handle={}, deadline={:#x}",
            handle_value, deadline
        );
        assert_eq!(core::mem::size_of::<PortPacket>(), 48);
        let port = self
            .thread
            .proc()
            .get_object_with_rights::<Port>(handle_value, Rights::READ)?;
        let packet = port.wait_async().await;
        warn!("port.wait: packet={:#x?}", packet);
        packet_res.write(packet)?;
        Ok(0)
    }

    pub fn sys_port_queue(
        &self,
        handle_value: HandleValue,
        packcet_in: UserInPtr<PortPacket>,
    ) -> ZxResult<usize> {
        // TODO when to return ZX_ERR_SHOULD_WAIT
        let port = self
            .thread
            .proc()
            .get_object_with_rights::<Port>(handle_value, Rights::WRITE)?;
        let packet = packcet_in.read()?;
        info!(
            "port.queue: handle={:#x}, packet={:?}",
            handle_value, packet
        );
        port.push(packet);
        Ok(0)
    }

    #[allow(unsafe_code)]
    pub async fn sys_futex_wait(
        &self,
        value_ptr: UserInPtr<AtomicI32>,
        current_value: i32,
        new_futex_owner: HandleValue,
        deadline: u64,
    ) -> ZxResult<usize> {
        info!(
            "futex.wait: value_ptr={:?}, current_value={:#x}, new_futex_owner={:#x}, deadline={:#x}",
            value_ptr, current_value, new_futex_owner, deadline
        );
        assert!(!value_ptr.is_null());
        let futex = Futex::new(unsafe { &*(value_ptr.as_ptr() as *const AtomicI32) });
        self.thread
            .proc()
            .add_futex(value_ptr.as_ptr() as usize, futex.clone());
        if new_futex_owner == INVALID_HANDLE {
            futex.wait_async(current_value).await?;
        } else {
            unimplemented!()
        }
        Ok(0)
    }

    pub async fn sys_nanosleep(&self, deadline: i64) -> ZxResult<usize> {
        if deadline <= 0 {
            // just yield current thread
            let yield_future = YieldFutureImpl { flag: false };
            yield_future.await;
        } else {
            unimplemented!()
        }
        Ok(0)
    }

    pub fn sys_futex_wake(&self, value_ptr: usize, count: u32) -> ZxResult<usize> {
        info!("futex.wake: value_ptr={:#x}, count={:#x}", value_ptr, count);
        let futex = self.thread.proc().get_futex(value_ptr)?;
        futex.wake(count as usize);
        Ok(0)
    }
}

struct YieldFutureImpl {
    flag: bool,
}

impl Future for YieldFutureImpl {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        warn!("POLL!!!");
        if self.flag {
            Poll::Ready(())
        } else {
            warn!("SLEEPING!!!");
            self.flag = true;
            cx.waker().clone().wake();
            Poll::Pending
        }
    }
}