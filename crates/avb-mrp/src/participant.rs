//! An MRP participant (IEEE 802.1Q-2022, 10.7) on one port for one
//! application: the Applicant and Registrar of each attribute, and the
//! join, leave, LeaveAll and periodic timers.
//!
//! Each attribute is named by its type and the leading octets of its
//! value (the stream ID of an MSRP talker or listener, an MVRP VLAN ID);
//! the rest of the value may change while it stays the same attribute.

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use core::time::Duration;

use crate::mrpdu::{self, Event, Format, Message};

/// JoinTime (10.7.11).
const JOIN_TIME: Duration = Duration::from_millis(200);
/// LeaveTime: between 600 ms and 1 s, the registrar keeps a withdrawn
/// attribute before taking it off (10.7.11).
const LEAVE_TIME: Duration = Duration::from_millis(800);
/// LeaveAllTime, and the spread added to it at random (10.7.11).
const LEAVE_ALL_TIME: Duration = Duration::from_secs(10);
const LEAVE_ALL_SPREAD: Duration = Duration::from_secs(5);
/// PeriodicTime (10.7.10).
const PERIODIC_TIME: Duration = Duration::from_secs(1);

/// The Applicant states (10.7.7): Very anxious, Anxious, Quiet and
/// Leaving, as an Observer, Passive member, New or Active member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Applicant {
    Vo,
    Vp,
    Vn,
    An,
    Aa,
    Qa,
    La,
    Ao,
    Qo,
    Ap,
    Qp,
    Lo,
}

/// The Registrar states (10.7.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Registrar {
    In,
    Lv,
    Mt,
}

/// What an Applicant sends at its next transmit opportunity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Send {
    New,
    Join,
    Leave,
}

/// The events that move the state machines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    New,
    Join,
    Leave,
    Received(Event),
    ReceivedLeaveAll,
    Transmit,
    TransmitLeaveAll,
    Periodic,
}

impl Applicant {
    /// Table 10-3, sending only from the states that must on shared media.
    fn step(self, step: Step) -> (Applicant, Option<Send>) {
        use Applicant::*;
        let next = match (step, self) {
            (Step::New, _) => Vn,
            (Step::Join, Vo | La | Lo) => Vp,
            (Step::Join, Ao) => Ap,
            (Step::Join, Qo) => Qp,
            (Step::Leave, Vp) => Vo,
            (Step::Leave, Vo | Vn | An) => Lo,
            (Step::Leave, Aa | Qa) => La,
            (Step::Leave, Ap) => Ao,
            (Step::Leave, Qp) => Qo,
            (Step::Received(Event::JoinIn), Vo) => Ao,
            (Step::Received(Event::JoinIn), Vp) => Ap,
            (Step::Received(Event::JoinIn | Event::In), Aa) => Qa,
            (Step::Received(Event::JoinIn), Ao) => Qo,
            (Step::Received(Event::JoinIn), Ap) => Qp,
            (Step::Received(Event::JoinMt | Event::Mt), Qa) => Aa,
            (Step::Received(Event::Lv) | Step::ReceivedLeaveAll, state) => match state {
                Vo | Ao | Qo => Lo,
                An => Vn,
                Aa | Qa | Ap | Qp => Vp,
                Lo => Vo,
                other => other,
            },
            (Step::Transmit | Step::TransmitLeaveAll, state) => {
                return match state {
                    Vp => (Aa, Some(Send::Join)),
                    Vn => (An, Some(Send::New)),
                    An => (Qa, Some(Send::New)),
                    Aa => (Qa, Some(Send::Join)),
                    La => (Vo, Some(Send::Leave)),
                    Lo => (Vo, None),
                    Qa if step == Step::TransmitLeaveAll => (Aa, Some(Send::Join)),
                    other => (other, None),
                };
            }
            (Step::Periodic, Qa) => Aa,
            (Step::Periodic, Qo) => Ao,
            (Step::Periodic, Qp) => Ap,
            (_, state) => state,
        };
        (next, None)
    }

    /// Whether it waits to send at the next transmit opportunity.
    fn anxious(self) -> bool {
        matches!(
            self,
            Applicant::Vp | Applicant::Vn | Applicant::An | Applicant::Aa | Applicant::La
        )
    }

    /// Whether it does not declare the attribute.
    fn observes(self) -> bool {
        matches!(
            self,
            Applicant::Vo | Applicant::Ao | Applicant::Qo | Applicant::Lo
        )
    }
}

/// A registration change on the port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Registration {
    /// A peer declares the attribute, or declares it with a new value or
    /// four-packed event.
    Joined {
        attribute_type: u8,
        value: Vec<u8>,
        four_packed: Option<u8>,
    },
    /// No peer declares the attribute any more.
    Left { attribute_type: u8, value: Vec<u8> },
}

#[derive(Debug, Clone)]
struct Attribute {
    applicant: Applicant,
    registrar: Registrar,
    /// When the Registrar takes the attribute off, while leaving.
    leave_at: Option<Duration>,
    /// What this participant declares, and the event it sends next.
    declared: Option<(Vec<u8>, Option<u8>)>,
    send: Option<Send>,
    /// What a peer declares.
    registered: Option<(Vec<u8>, Option<u8>)>,
}

impl Attribute {
    fn new() -> Self {
        Attribute {
            applicant: Applicant::Vo,
            registrar: Registrar::Mt,
            leave_at: None,
            declared: None,
            send: None,
            registered: None,
        }
    }
}

/// One application's participant on one port. Feed it received MRPDUs
/// and the time; take back MRPDUs to send, registration changes and the
/// next deadline. Times are durations from any start the caller keeps.
pub struct Participant {
    format: Format,
    attributes: BTreeMap<(u8, Vec<u8>), Attribute>,
    join_at: Option<Duration>,
    leave_all_at: Duration,
    periodic_at: Duration,
    /// The next MRPDU carries LeaveAll.
    leave_all_due: bool,
    random: u64,
    outgoing: VecDeque<Vec<u8>>,
    events: VecDeque<Registration>,
}

impl Participant {
    /// A participant for the application `format` describes; `seed`
    /// spreads its timers from others'.
    pub fn new(format: Format, now: Duration, seed: u64) -> Self {
        let mut participant = Participant {
            format,
            attributes: BTreeMap::new(),
            join_at: None,
            leave_all_at: now,
            periodic_at: now + PERIODIC_TIME,
            leave_all_due: false,
            random: seed | 1,
            outgoing: VecDeque::new(),
            events: VecDeque::new(),
        };
        participant.leave_all_at = now + participant.leave_all_period();
        participant
    }

    fn next_random(&mut self) -> u64 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        self.random
    }

    fn leave_all_period(&mut self) -> Duration {
        let spread = self.next_random() % LEAVE_ALL_SPREAD.as_millis() as u64;
        LEAVE_ALL_TIME + Duration::from_millis(spread)
    }

    fn key(&self, attribute_type: u8, value: &[u8]) -> (u8, Vec<u8>) {
        let length = (self.format.key_length)(attribute_type).min(value.len());
        (attribute_type, Vec::from(&value[..length]))
    }

    /// Asks for a transmit opportunity within JoinTime.
    fn arm_join(&mut self, now: Duration) {
        if self.join_at.is_none() {
            let spread = self.next_random() % JOIN_TIME.as_millis() as u64;
            self.join_at = Some(now + Duration::from_millis(spread));
        }
    }

    /// Declares the attribute `value` names, with `four_packed` for the
    /// types that carry one. Declaring it again with a different value or
    /// four-packed event withdraws the old one first.
    pub fn declare(
        &mut self,
        now: Duration,
        attribute_type: u8,
        value: Vec<u8>,
        four_packed: Option<u8>,
    ) {
        let key = self.key(attribute_type, &value);
        let attribute = self.attributes.entry(key).or_insert_with(Attribute::new);
        let wanted = Some((value, four_packed));
        let same = attribute.declared == wanted;
        let leaving = attribute.applicant == Applicant::La;
        if same && !leaving && !attribute.applicant.observes() {
            return;
        }
        if !same
            && let Some((old, old_four)) = attribute.declared.clone()
            && !attribute.applicant.observes()
        {
            // A new value replaces the old: the old leaves at once.
            let message = Message {
                attribute_type,
                leave_all: false,
                values: Vec::from([(old, Event::Lv, old_four)]),
            };
            self.outgoing
                .push_back(mrpdu::encode(&[message], &self.format));
            attribute.applicant = Applicant::Vo;
        }
        attribute.declared = wanted;
        let step = if attribute.applicant.observes() {
            Step::New
        } else {
            Step::Join
        };
        attribute.applicant = attribute.applicant.step(step).0;
        self.arm_join(now);
    }

    /// Withdraws the attribute `key` names: the leading octets of its
    /// value, as the format counts them.
    pub fn withdraw(&mut self, now: Duration, attribute_type: u8, key: &[u8]) {
        let key = self.key(attribute_type, key);
        if let Some(attribute) = self.attributes.get_mut(&key)
            && attribute.declared.is_some()
        {
            attribute.applicant = attribute.applicant.step(Step::Leave).0;
            self.arm_join(now);
        }
    }

    /// Handles a received MRPDU's payload.
    pub fn handle_pdu(&mut self, now: Duration, pdu: &[u8]) {
        let decoded = mrpdu::decode(pdu, &self.format);
        for attribute_type in &decoded.leave_all {
            self.leave_all(now, Some(*attribute_type));
        }
        if !decoded.leave_all.is_empty() {
            // Another participant's LeaveAll stands for ours too.
            self.leave_all_at = now + self.leave_all_period();
        }
        for received in decoded.values {
            let key = self.key(received.attribute_type, &received.value);
            let attribute = self.attributes.entry(key).or_insert_with(Attribute::new);
            attribute.applicant = attribute
                .applicant
                .step(Step::Received(received.event))
                .0;
            let before = attribute.registrar;
            match received.event {
                Event::New | Event::JoinIn | Event::JoinMt => {
                    attribute.registrar = Registrar::In;
                    attribute.leave_at = None;
                    let registered = Some((received.value.clone(), received.four_packed));
                    let changed = before == Registrar::Mt || attribute.registered != registered;
                    attribute.registered = registered;
                    if changed {
                        self.events.push_back(Registration::Joined {
                            attribute_type: received.attribute_type,
                            value: received.value,
                            four_packed: received.four_packed,
                        });
                    }
                }
                Event::Lv => {
                    if before != Registrar::Mt {
                        attribute.registrar = Registrar::Lv;
                        attribute.leave_at = Some(now + LEAVE_TIME);
                    }
                }
                // The sender does not declare it: In and Mt only say
                // what it has registered (10.7.8).
                Event::In | Event::Mt => {}
            }
            if attribute.applicant.anxious() {
                self.arm_join(now);
            }
        }
    }

    /// LeaveAll for one attribute type, or every type when `None`: every
    /// registration starts leaving, and every declaration is made again.
    fn leave_all(&mut self, now: Duration, attribute_type: Option<u8>) {
        let mut anxious = false;
        for ((kind, _), attribute) in self.attributes.iter_mut() {
            if attribute_type.is_some_and(|wanted| wanted != *kind) {
                continue;
            }
            attribute.applicant = attribute.applicant.step(Step::ReceivedLeaveAll).0;
            if attribute.registrar == Registrar::In {
                attribute.registrar = Registrar::Lv;
                attribute.leave_at = Some(now + LEAVE_TIME);
            }
            anxious |= attribute.applicant.anxious();
        }
        if anxious {
            self.arm_join(now);
        }
    }

    /// Handles the timers due by `now`.
    pub fn handle_timeout(&mut self, now: Duration) {
        // Registrations that have finished leaving.
        let mut left = Vec::new();
        for ((kind, _), attribute) in self.attributes.iter_mut() {
            if attribute.leave_at.is_some_and(|at| at <= now) {
                attribute.leave_at = None;
                attribute.registrar = Registrar::Mt;
                if let Some((value, _)) = attribute.registered.take() {
                    left.push(Registration::Left {
                        attribute_type: *kind,
                        value,
                    });
                }
            }
        }
        self.events.extend(left);
        if now >= self.leave_all_at {
            self.leave_all_at = now + self.leave_all_period();
            self.leave_all_due = true;
            self.leave_all(now, None);
            self.join_at = Some(now);
        }
        if now >= self.periodic_at {
            self.periodic_at = now + PERIODIC_TIME;
            let mut anxious = false;
            for attribute in self.attributes.values_mut() {
                attribute.applicant = attribute.applicant.step(Step::Periodic).0;
                anxious |= attribute.applicant.anxious();
            }
            if anxious {
                self.arm_join(now);
            }
        }
        if self.join_at.is_some_and(|at| at <= now) {
            self.join_at = None;
            self.transmit();
        }
        // Attributes neither declared nor registered are forgotten.
        self.attributes.retain(|_, attribute| {
            attribute.declared.is_some() && attribute.applicant != Applicant::Vo
                || attribute.registrar != Registrar::Mt
                || attribute.applicant.anxious()
        });
    }

    /// Sends what every anxious Applicant waits to send, with LeaveAll
    /// when it is due.
    fn transmit(&mut self) {
        let step = if self.leave_all_due {
            Step::TransmitLeaveAll
        } else {
            Step::Transmit
        };
        let mut messages: Vec<Message> = Vec::new();
        for ((kind, _), attribute) in self.attributes.iter_mut() {
            let (next, send) = attribute.applicant.step(step);
            attribute.applicant = next;
            attribute.send = send;
            let Some(send) = send else {
                continue;
            };
            let Some((value, four_packed)) = attribute.declared.clone() else {
                continue;
            };
            let event = match send {
                Send::New => Event::New,
                Send::Join if attribute.registrar == Registrar::In => Event::JoinIn,
                Send::Join => Event::JoinMt,
                Send::Leave => Event::Lv,
            };
            if send == Send::Leave {
                attribute.declared = None;
            }
            match messages.iter_mut().find(|message| message.attribute_type == *kind) {
                Some(message) => message.values.push((value, event, four_packed)),
                None => messages.push(Message {
                    attribute_type: *kind,
                    leave_all: self.leave_all_due,
                    values: Vec::from([(value, event, four_packed)]),
                }),
            }
        }
        if self.leave_all_due && messages.is_empty() {
            // LeaveAll alone still goes out, for the first type known.
            if let Some(kind) = (1..=u8::MAX).find(|kind| (self.format.first_value_length)(*kind).is_some()) {
                messages.push(Message {
                    attribute_type: kind,
                    leave_all: true,
                    values: Vec::new(),
                });
            }
        }
        self.leave_all_due = false;
        if !messages.is_empty() {
            self.outgoing.push_back(mrpdu::encode(&messages, &self.format));
        }
    }

    /// The next MRPDU payload to send.
    pub fn poll_transmit(&mut self) -> Option<Vec<u8>> {
        self.outgoing.pop_front()
    }

    /// The next registration change.
    pub fn poll_event(&mut self) -> Option<Registration> {
        self.events.pop_front()
    }

    /// When `handle_timeout` should next be called.
    pub fn poll_timeout(&self) -> Duration {
        let leaving = self
            .attributes
            .values()
            .filter_map(|attribute| attribute.leave_at)
            .min();
        [Some(self.leave_all_at), Some(self.periodic_at), self.join_at, leaving]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(self.leave_all_at)
    }

    /// The attributes peers declare now: type, value and four-packed
    /// event.
    pub fn registered(&self) -> impl Iterator<Item = (u8, &[u8], Option<u8>)> {
        self.attributes.iter().filter_map(|((kind, _), attribute)| {
            let (value, four_packed) = attribute.registered.as_ref()?;
            Some((*kind, value.as_slice(), *four_packed))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msrp::{self, ListenerState, TalkerDeclaration};
    use crate::MacAddress;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn advertise() -> Vec<u8> {
        msrp::talker_value(&TalkerDeclaration {
            stream_id: 0x9c6b_0030_9a2b_0001,
            destination: MacAddress([0x91, 0xe0, 0xf0, 0x00, 0x12, 0x34]),
            vlan_id: 2,
            max_frame_size: 224,
            max_interval_frames: 1,
            priority: 3,
            rank: true,
            accumulated_latency: 0,
            failure: None,
            event: Event::New,
        })
    }

    /// Runs both participants from `from` to `to`, handing each one's
    /// MRPDUs to the other, and returns the registration changes each saw.
    fn run(
        one: &mut Participant,
        other: &mut Participant,
        from: u64,
        to: u64,
    ) -> (Vec<Registration>, Vec<Registration>) {
        let mut seen = (Vec::new(), Vec::new());
        for now in (from..=to).step_by(10).map(ms) {
            one.handle_timeout(now);
            other.handle_timeout(now);
            while let Some(pdu) = one.poll_transmit() {
                other.handle_pdu(now, &pdu);
            }
            while let Some(pdu) = other.poll_transmit() {
                one.handle_pdu(now, &pdu);
            }
            seen.0.extend(core::iter::from_fn(|| one.poll_event()));
            seen.1.extend(core::iter::from_fn(|| other.poll_event()));
        }
        seen
    }

    #[test]
    fn a_talker_and_a_listener_register_each_other() {
        let mut talker = Participant::new(msrp::FORMAT, ms(0), 1);
        let mut listener = Participant::new(msrp::FORMAT, ms(0), 2);
        talker.declare(ms(0), msrp::attribute::TALKER_ADVERTISE, advertise(), None);
        let (_, heard) = run(&mut talker, &mut listener, 0, 500);
        assert_eq!(
            heard,
            [Registration::Joined {
                attribute_type: msrp::attribute::TALKER_ADVERTISE,
                value: advertise(),
                four_packed: None,
            }]
        );
        let stream = advertise()[..8].to_vec();
        let ready = Some(ListenerState::Ready.number());
        listener.declare(ms(500), msrp::attribute::LISTENER, stream.clone(), ready);
        let (heard, _) = run(&mut talker, &mut listener, 500, 1000);
        assert_eq!(
            heard,
            [Registration::Joined {
                attribute_type: msrp::attribute::LISTENER,
                value: stream.clone(),
                four_packed: ready,
            }]
        );
        // LeaveAll cycles come and go without either losing the other.
        let (one, two) = run(&mut talker, &mut listener, 1000, 40_000);
        assert!(one.is_empty() && two.is_empty(), "{one:?} {two:?}");
        assert_eq!(talker.registered().count(), 1);
        assert_eq!(listener.registered().count(), 1);
        // Withdrawn, the listener is taken off within a leave time.
        listener.withdraw(ms(40_000), msrp::attribute::LISTENER, &stream);
        let (heard, _) = run(&mut talker, &mut listener, 40_000, 41_500);
        assert_eq!(
            heard,
            [Registration::Left {
                attribute_type: msrp::attribute::LISTENER,
                value: stream,
            }]
        );
    }

    #[test]
    fn a_new_four_packed_event_registers_again() {
        let mut talker = Participant::new(msrp::FORMAT, ms(0), 3);
        let mut listener = Participant::new(msrp::FORMAT, ms(0), 4);
        let stream = advertise()[..8].to_vec();
        let asking = Some(ListenerState::AskingFailed.number());
        listener.declare(ms(0), msrp::attribute::LISTENER, stream.clone(), asking);
        run(&mut talker, &mut listener, 0, 500);
        let ready = Some(ListenerState::Ready.number());
        listener.declare(ms(500), msrp::attribute::LISTENER, stream.clone(), ready);
        let (heard, _) = run(&mut talker, &mut listener, 500, 1000);
        assert_eq!(
            heard.last(),
            Some(&Registration::Joined {
                attribute_type: msrp::attribute::LISTENER,
                value: stream,
                four_packed: ready,
            })
        );
    }

    #[test]
    fn a_peer_that_falls_silent_is_taken_off_after_its_leave_all() {
        let mut talker = Participant::new(msrp::FORMAT, ms(0), 5);
        let mut listener = Participant::new(msrp::FORMAT, ms(0), 6);
        talker.declare(ms(0), msrp::attribute::TALKER_ADVERTISE, advertise(), None);
        run(&mut talker, &mut listener, 0, 500);
        // The talker's frames stop arriving: the listener's own LeaveAll
        // starts its registration leaving, and nothing renews it.
        let mut left = Vec::new();
        for now in (500..=20_000).step_by(10).map(ms) {
            listener.handle_timeout(now);
            while listener.poll_transmit().is_some() {}
            left.extend(core::iter::from_fn(|| listener.poll_event()));
        }
        assert!(matches!(left[..], [Registration::Left { .. }]), "{left:?}");
    }
}
